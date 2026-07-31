use crate::infrastructure::codex_runner::{CodexInvocation, filter_inherited_environment};
use std::ffi::OsString;
use std::fmt;
use std::fs::OpenOptions;
use std::future::Future;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tokio::sync::watch;

pub const MAX_PROCESS_OUTPUT_BYTES: usize = 1024 * 1024;
pub const MAX_PROCESS_STDIN_BYTES: usize = 1024 * 1024;
const PROCESS_TERMINATION_GRACE: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const MAX_JOB_PROCESS_IDS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProcessError {
    #[error("无法创建 Windows Job Object")]
    JobUnavailable,
    #[error("无法将子进程加入 Windows Job Object")]
    JobAssignment,
    #[error("子进程启动失败")]
    ProcessStart,
    #[error("子进程无法从安全挂起状态恢复")]
    ProcessResume,
    #[error("子进程输出超过安全上限")]
    OutputTooLarge,
    #[error("子进程超时")]
    Timeout,
    #[error("子进程已取消")]
    Cancelled,
    #[error("子进程树未能安全终止")]
    ProcessTreeTermination,
    #[error("读取子进程输出失败")]
    OutputRead,
    #[error("子进程输入超过安全上限")]
    InputTooLarge,
    #[error("写入子进程输入失败")]
    InputWrite,
}

pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl fmt::Debug for ProcessOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProcessOutput")
            .field("exit_code", &self.exit_code)
            .field("stdout_len", &self.stdout.len())
            .field("stderr_len", &self.stderr.len())
            .finish()
    }
}

#[derive(Clone)]
pub struct ProcessInvocation {
    pub executable: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
    pub workdir: PathBuf,
    pub stdin: Option<Vec<u8>>,
    pub stdout_file: Option<PathBuf>,
}

impl fmt::Debug for ProcessInvocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let env_names = self
            .env
            .iter()
            .map(|(name, _)| name.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        formatter
            .debug_struct("ProcessInvocation")
            .field("executable", &self.executable)
            .field("arg_count", &self.args.len())
            .field("env_names", &env_names)
            .field("workdir", &self.workdir)
            .field("stdin_len", &self.stdin.as_ref().map(Vec::len))
            .field("stdout_file", &self.stdout_file)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessStream {
    Stdout,
    Stderr,
}

pub trait ProcessEventSink: Send + Sync + 'static {
    fn on_output(&self, stream: ProcessStream, bytes: &[u8]);
}

pub(crate) type CodexProcessError = ProcessError;
pub(crate) type CodexProcessOutput = ProcessOutput;

pub(crate) trait CodexProcessBackend: Send + Sync {
    fn run(
        &self,
        invocation: CodexInvocation,
        timeout: Duration,
        cancel: watch::Receiver<bool>,
    ) -> Pin<Box<dyn Future<Output = Result<CodexProcessOutput, CodexProcessError>> + Send>>;
}

pub(crate) trait JobController: Send {
    fn assign(&self, child: &Child) -> Result<(), CodexProcessError>;
    fn terminate(&self) -> Result<(), CodexProcessError>;
    fn active_processes(&self) -> Result<u32, CodexProcessError>;

    fn process_ids(&self) -> Result<Vec<u32>, CodexProcessError> {
        Ok(Vec::new())
    }
}

pub(crate) trait JobFactory: Send + Sync {
    fn create(&self) -> Result<Box<dyn JobController>, CodexProcessError>;
}

#[derive(Clone)]
pub struct SafeProcessRunner {
    job_factory: Arc<dyn JobFactory>,
}

impl Default for SafeProcessRunner {
    fn default() -> Self {
        Self {
            job_factory: Arc::new(SystemJobFactory),
        }
    }
}

impl SafeProcessRunner {
    #[cfg(test)]
    fn with_job_factory(job_factory: Arc<dyn JobFactory>) -> Self {
        Self { job_factory }
    }

    pub async fn run(
        &self,
        invocation: ProcessInvocation,
        timeout: Duration,
        cancel: watch::Receiver<bool>,
        event_sink: Option<Arc<dyn ProcessEventSink>>,
    ) -> Result<ProcessOutput, ProcessError> {
        let job_factory = Arc::clone(&self.job_factory);
        tokio::task::spawn_blocking(move || {
            run_blocking(invocation, timeout, cancel, job_factory, event_sink)
        })
        .await
        .map_err(|_| ProcessError::ProcessStart)?
    }
}

#[derive(Clone, Default)]
pub(crate) struct SystemCodexProcessBackend {
    runner: SafeProcessRunner,
}

impl SystemCodexProcessBackend {
    #[cfg(test)]
    pub(crate) fn with_job_factory(job_factory: Arc<dyn JobFactory>) -> Self {
        Self {
            runner: SafeProcessRunner::with_job_factory(job_factory),
        }
    }
}

impl CodexProcessBackend for SystemCodexProcessBackend {
    fn run(
        &self,
        invocation: CodexInvocation,
        timeout: Duration,
        cancel: watch::Receiver<bool>,
    ) -> Pin<Box<dyn Future<Output = Result<CodexProcessOutput, CodexProcessError>> + Send>> {
        let mut env = filter_inherited_environment(std::env::vars_os());
        env.extend(invocation.env);
        let invocation = ProcessInvocation {
            executable: invocation.executable,
            args: invocation.args,
            env,
            workdir: invocation.workdir,
            stdin: None,
            stdout_file: None,
        };
        let runner = self.runner.clone();
        Box::pin(async move { runner.run(invocation, timeout, cancel, None).await })
    }
}

fn run_blocking(
    invocation: ProcessInvocation,
    timeout: Duration,
    cancel: watch::Receiver<bool>,
    job_factory: Arc<dyn JobFactory>,
    event_sink: Option<Arc<dyn ProcessEventSink>>,
) -> Result<CodexProcessOutput, CodexProcessError> {
    if invocation
        .stdin
        .as_ref()
        .is_some_and(|stdin| stdin.len() > MAX_PROCESS_STDIN_BYTES)
    {
        return Err(CodexProcessError::InputTooLarge);
    }
    let stdout_to_file = invocation.stdout_file.is_some();
    let stdout = match &invocation.stdout_file {
        Some(path) => Stdio::from(
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path)
                .map_err(|_| CodexProcessError::ProcessStart)?,
        ),
        None => Stdio::piped(),
    };
    let mut command = Command::new(&invocation.executable);
    command
        .args(&invocation.args)
        .current_dir(&invocation.workdir)
        .stdin(if invocation.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(stdout)
        .stderr(Stdio::piped())
        .env_clear();
    for (name, value) in invocation.env {
        command.env(name, value);
    }
    configure_process_creation(&mut command);

    let job = job_factory.create()?;
    let mut child = command
        .spawn()
        .map_err(|_| CodexProcessError::ProcessStart)?;
    if let Err(error) = job.assign(&child) {
        let _ = child.kill();
        let _ = child.wait();
        let _ = job.terminate();
        return Err(error);
    }
    if let Err(error) = resume_child(&child) {
        let _ = job.terminate();
        let _ = child.wait();
        return Err(error);
    }

    let stdout: Box<dyn Read + Send> = match child.stdout.take() {
        Some(stdout) => Box::new(stdout),
        None if stdout_to_file => Box::new(std::io::empty()),
        None => return Err(CodexProcessError::ProcessStart),
    };
    let stderr = child.stderr.take().ok_or(CodexProcessError::ProcessStart)?;
    let (output_sender, output_receiver) = mpsc::channel();
    let stdout_thread = spawn_reader(
        stdout,
        OutputStream::Stdout,
        output_sender.clone(),
        event_sink.clone(),
    );
    let stderr_thread = spawn_reader(stderr, OutputStream::Stderr, output_sender, event_sink);
    if let Some(stdin) = invocation.stdin {
        let write_result = child
            .stdin
            .take()
            .ok_or(CodexProcessError::InputWrite)
            .and_then(|mut child_stdin| {
                child_stdin
                    .write_all(&stdin)
                    .and_then(|()| child_stdin.flush())
                    .map_err(|_| CodexProcessError::InputWrite)
            });
        if let Err(error) = write_result {
            let _ = terminate_job_and_wait(&*job, &mut child);
            join_reader(stdout_thread);
            join_reader(stderr_thread);
            return Err(error);
        }
    }
    let deadline = Instant::now() + timeout;
    let mut stdout_result = None;
    let mut stderr_result = None;
    let mut termination_reason = None;
    let mut exit_status = None;

    loop {
        if *cancel.borrow() {
            termination_reason = Some(CodexProcessError::Cancelled);
            break;
        }
        if Instant::now() >= deadline {
            termination_reason = Some(CodexProcessError::Timeout);
            break;
        }
        while let Ok(message) = output_receiver.try_recv() {
            match message {
                ReaderMessage::Stdout(result) => stdout_result = Some(result),
                ReaderMessage::Stderr(result) => stderr_result = Some(result),
            }
        }
        if let Some(error) = reader_error(stdout_result.as_ref(), stderr_result.as_ref()) {
            termination_reason = Some(error);
            break;
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                exit_status = Some(status);
                break;
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(_) => {
                termination_reason = Some(CodexProcessError::ProcessStart);
                break;
            }
        }
    }

    if let Some(reason) = termination_reason {
        let terminated = terminate_job_and_wait(&*job, &mut child);
        join_reader(stdout_thread);
        join_reader(stderr_thread);
        if !terminated {
            return Err(CodexProcessError::ProcessTreeTermination);
        }
        return Err(reason);
    }

    let active_deadline = Instant::now() + PROCESS_TERMINATION_GRACE;
    while job.active_processes()? > 0 && Instant::now() < active_deadline {
        thread::sleep(POLL_INTERVAL);
    }
    if job.active_processes()? > 0 {
        let _ = job.terminate();
        join_reader(stdout_thread);
        join_reader(stderr_thread);
        return Err(CodexProcessError::ProcessTreeTermination);
    }
    join_reader(stdout_thread);
    join_reader(stderr_thread);
    while let Ok(message) = output_receiver.try_recv() {
        match message {
            ReaderMessage::Stdout(result) => stdout_result = Some(result),
            ReaderMessage::Stderr(result) => stderr_result = Some(result),
        }
    }
    let stdout = stdout_result.ok_or(CodexProcessError::OutputRead)??;
    let stderr = stderr_result.ok_or(CodexProcessError::OutputRead)??;
    Ok(CodexProcessOutput {
        exit_code: exit_status.and_then(|status| status.code()),
        stdout,
        stderr,
    })
}

fn configure_process_creation(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        command.creation_flags(CREATE_NO_WINDOW | CREATE_SUSPENDED);
    }
}

#[cfg(windows)]
fn resume_child(child: &Child) -> Result<(), CodexProcessError> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows_sys::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Err(CodexProcessError::ProcessResume);
    }
    let mut entry = THREADENTRY32 {
        dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
        ..Default::default()
    };
    let mut has_entry = unsafe { Thread32First(snapshot, &mut entry) } != 0;
    let mut resumed = 0_u32;
    while has_entry {
        if entry.th32OwnerProcessID == child.id() {
            let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID) };
            if thread.is_null() {
                unsafe { CloseHandle(snapshot) };
                return Err(CodexProcessError::ProcessResume);
            }
            let previous_count = unsafe { ResumeThread(thread) };
            unsafe { CloseHandle(thread) };
            if previous_count == u32::MAX {
                unsafe { CloseHandle(snapshot) };
                return Err(CodexProcessError::ProcessResume);
            }
            resumed += 1;
        }
        has_entry = unsafe { Thread32Next(snapshot, &mut entry) } != 0;
    }
    unsafe { CloseHandle(snapshot) };
    (resumed > 0)
        .then_some(())
        .ok_or(CodexProcessError::ProcessResume)
}

#[cfg(not(windows))]
fn resume_child(_child: &Child) -> Result<(), CodexProcessError> {
    Ok(())
}

fn terminate_job_and_wait(job: &dyn JobController, child: &mut Child) -> bool {
    let initial_process_ids = job.process_ids();
    let terminated = job.terminate().is_ok();
    if !terminated {
        let process_id = child.id();
        let _ = child.kill();
        let _ = child.wait();
        fallback_terminate_process_tree(process_id);
        return false;
    }
    let mut tracked_process_ids = match initial_process_ids {
        Ok(process_ids) => process_ids,
        Err(_) => {
            let _ = child.wait();
            return false;
        }
    };
    let _ = child.wait();
    let deadline = Instant::now() + PROCESS_TERMINATION_GRACE;
    loop {
        let process_ids = match job.process_ids() {
            Ok(process_ids) => process_ids,
            Err(_) => return false,
        };
        tracked_process_ids.extend(process_ids);
        tracked_process_ids.sort_unstable();
        tracked_process_ids.dedup();
        let active = job.active_processes().ok();
        if active == Some(0) && process_ids_have_exited(&tracked_process_ids) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(windows)]
fn process_ids_have_exited(process_ids: &[u32]) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INVALID_PARAMETER, GetLastError, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject,
    };

    process_ids.iter().all(|process_id| {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, *process_id) };
        if handle.is_null() {
            return unsafe { GetLastError() } == ERROR_INVALID_PARAMETER;
        }
        let status = unsafe { WaitForSingleObject(handle, 0) };
        unsafe { CloseHandle(handle) };
        status == WAIT_OBJECT_0
    })
}

#[cfg(not(windows))]
fn process_ids_have_exited(_process_ids: &[u32]) -> bool {
    true
}

fn fallback_terminate_process_tree(process_id: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill.exe")
            .args(["/PID", &process_id.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    let _ = process_id;
}

#[derive(Clone, Copy)]
enum OutputStream {
    Stdout,
    Stderr,
}

impl From<OutputStream> for ProcessStream {
    fn from(value: OutputStream) -> Self {
        match value {
            OutputStream::Stdout => Self::Stdout,
            OutputStream::Stderr => Self::Stderr,
        }
    }
}

enum ReaderMessage {
    Stdout(Result<Vec<u8>, CodexProcessError>),
    Stderr(Result<Vec<u8>, CodexProcessError>),
}

fn spawn_reader<R>(
    reader: R,
    stream: OutputStream,
    sender: mpsc::Sender<ReaderMessage>,
    event_sink: Option<Arc<dyn ProcessEventSink>>,
) -> thread::JoinHandle<()>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let result = read_bounded(reader, stream, event_sink.as_deref());
        let message = match stream {
            OutputStream::Stdout => ReaderMessage::Stdout(result),
            OutputStream::Stderr => ReaderMessage::Stderr(result),
        };
        let _ = sender.send(message);
    })
}

fn read_bounded<R: Read>(
    mut reader: R,
    stream: OutputStream,
    event_sink: Option<&dyn ProcessEventSink>,
) -> Result<Vec<u8>, CodexProcessError> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| CodexProcessError::OutputRead)?;
        if read == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(read) > MAX_PROCESS_OUTPUT_BYTES {
            return Err(CodexProcessError::OutputTooLarge);
        }
        if let Some(sink) = event_sink {
            sink.on_output(stream.into(), &buffer[..read]);
        }
        output.extend_from_slice(&buffer[..read]);
    }
}

fn reader_error(
    stdout: Option<&Result<Vec<u8>, CodexProcessError>>,
    stderr: Option<&Result<Vec<u8>, CodexProcessError>>,
) -> Option<CodexProcessError> {
    [stdout, stderr]
        .into_iter()
        .flatten()
        .find_map(|result| result.as_ref().err().copied())
}

fn join_reader(thread: thread::JoinHandle<()>) {
    let _ = thread.join();
}

#[cfg(windows)]
#[derive(Clone, Copy, Default)]
struct SystemJobFactory;

#[cfg(windows)]
impl JobFactory for SystemJobFactory {
    fn create(&self) -> Result<Box<dyn JobController>, CodexProcessError> {
        Ok(Box::new(WindowsJobController::new()?))
    }
}

#[cfg(not(windows))]
#[derive(Clone, Copy, Default)]
struct SystemJobFactory;

#[cfg(not(windows))]
impl JobFactory for SystemJobFactory {
    fn create(&self) -> Result<Box<dyn JobController>, CodexProcessError> {
        Err(CodexProcessError::JobUnavailable)
    }
}

#[cfg(windows)]
struct WindowsJobController {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
unsafe impl Send for WindowsJobController {}

#[cfg(windows)]
impl WindowsJobController {
    fn new() -> Result<Self, CodexProcessError> {
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(CodexProcessError::JobUnavailable);
        }
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } != 0;
        if !configured {
            unsafe { windows_sys::Win32::Foundation::CloseHandle(handle) };
            return Err(CodexProcessError::JobUnavailable);
        }
        Ok(Self { handle })
    }
}

#[cfg(windows)]
impl JobController for WindowsJobController {
    fn assign(&self, child: &Child) -> Result<(), CodexProcessError> {
        use std::os::windows::io::AsRawHandle;
        let assigned = unsafe {
            windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(
                self.handle,
                child.as_raw_handle().cast(),
            )
        } != 0;
        assigned
            .then_some(())
            .ok_or(CodexProcessError::JobAssignment)
    }

    fn terminate(&self) -> Result<(), CodexProcessError> {
        let terminated =
            unsafe { windows_sys::Win32::System::JobObjects::TerminateJobObject(self.handle, 1) }
                != 0;
        terminated
            .then_some(())
            .ok_or(CodexProcessError::ProcessTreeTermination)
    }

    fn active_processes(&self) -> Result<u32, CodexProcessError> {
        use windows_sys::Win32::System::JobObjects::{
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION, JobObjectBasicAccountingInformation,
            QueryInformationJobObject,
        };
        let mut info = JOBOBJECT_BASIC_ACCOUNTING_INFORMATION::default();
        let queried = unsafe {
            QueryInformationJobObject(
                self.handle,
                JobObjectBasicAccountingInformation,
                (&mut info as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
                std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                std::ptr::null_mut(),
            )
        } != 0;
        queried
            .then_some(info.ActiveProcesses)
            .ok_or(CodexProcessError::ProcessTreeTermination)
    }

    fn process_ids(&self) -> Result<Vec<u32>, CodexProcessError> {
        use windows_sys::Win32::System::JobObjects::{
            JOBOBJECT_BASIC_PROCESS_ID_LIST, JobObjectBasicProcessIdList, QueryInformationJobObject,
        };

        let byte_len = std::mem::size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()
            + (MAX_JOB_PROCESS_IDS - 1) * std::mem::size_of::<usize>();
        let word_len = byte_len.div_ceil(std::mem::size_of::<usize>());
        let mut buffer = vec![0_usize; word_len];
        let queried = unsafe {
            QueryInformationJobObject(
                self.handle,
                JobObjectBasicProcessIdList,
                buffer.as_mut_ptr().cast(),
                byte_len as u32,
                std::ptr::null_mut(),
            )
        } != 0;
        if !queried {
            return Err(CodexProcessError::ProcessTreeTermination);
        }
        let info = unsafe { &*(buffer.as_ptr().cast::<JOBOBJECT_BASIC_PROCESS_ID_LIST>()) };
        let count = info.NumberOfProcessIdsInList as usize;
        if count > MAX_JOB_PROCESS_IDS {
            return Err(CodexProcessError::ProcessTreeTermination);
        }
        let process_ids = unsafe { std::slice::from_raw_parts(info.ProcessIdList.as_ptr(), count) };
        process_ids
            .iter()
            .copied()
            .map(|process_id| {
                u32::try_from(process_id).map_err(|_| CodexProcessError::ProcessTreeTermination)
            })
            .collect()
    }
}

#[cfg(windows)]
impl Drop for WindowsJobController {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.handle) };
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::infrastructure::codex_runner::CodexInvocation;
    use serial_test::serial;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    const PROCESS_TREE_TEST_TIMEOUT: Duration = Duration::from_secs(30);

    struct CreateFailingJobFactory;

    impl JobFactory for CreateFailingJobFactory {
        fn create(&self) -> Result<Box<dyn JobController>, CodexProcessError> {
            Err(CodexProcessError::JobUnavailable)
        }
    }

    struct AssignFailingJobFactory;

    impl JobFactory for AssignFailingJobFactory {
        fn create(&self) -> Result<Box<dyn JobController>, CodexProcessError> {
            Ok(Box::new(AssignFailingJob))
        }
    }

    struct AssignFailingJob;

    impl JobController for AssignFailingJob {
        fn assign(&self, _child: &Child) -> Result<(), CodexProcessError> {
            Err(CodexProcessError::JobAssignment)
        }

        fn terminate(&self) -> Result<(), CodexProcessError> {
            Ok(())
        }

        fn active_processes(&self) -> Result<u32, CodexProcessError> {
            Ok(0)
        }
    }

    fn is_process_running(process_id: u32) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
            WaitForSingleObject,
        };
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                process_id,
            )
        };
        if handle.is_null() {
            return false;
        }
        let status = unsafe { WaitForSingleObject(handle, 0) };
        unsafe { CloseHandle(handle) };
        status == WAIT_TIMEOUT
    }

    fn powershell() -> PathBuf {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
    }

    fn invocation(workdir: &Path, script: impl Into<OsString>) -> CodexInvocation {
        CodexInvocation {
            executable: powershell(),
            args: vec![
                OsString::from("-NoLogo"),
                OsString::from("-NoProfile"),
                OsString::from("-NonInteractive"),
                OsString::from("-Command"),
                script.into(),
            ],
            env: Vec::new(),
            workdir: workdir.to_owned(),
        }
    }

    async fn wait_for_process_id(path: &Path, status_path: &Path, error_path: &Path) -> u32 {
        let deadline = Instant::now() + PROCESS_TREE_TEST_TIMEOUT;
        while Instant::now() < deadline {
            if let Ok(value) = fs::read_to_string(path)
                && let Ok(process_id) = value.trim().parse::<u32>()
            {
                return process_id;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let status = fs::read_to_string(status_path)
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|_| "<missing>".to_owned());
        let error = fs::read_to_string(error_path)
            .map(|value| value.trim().to_owned())
            .unwrap_or_else(|_| "<missing>".to_owned());
        panic!(
            "child process id was not written in time; parent status: {status}; parent error: {error}"
        );
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn captures_bounded_stdout_and_stderr_without_debug_exposure() {
        let directory = tempfile::tempdir().unwrap();
        let (_sender, cancel) = tokio::sync::watch::channel(false);
        let output = SystemCodexProcessBackend::default()
            .run(
                invocation(
                    directory.path(),
                    "[Console]::Out.Write('safe-out'); [Console]::Error.Write('safe-err')",
                ),
                Duration::from_secs(5),
                cancel,
            )
            .await
            .unwrap();

        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout, b"safe-out");
        assert_eq!(output.stderr, b"safe-err");
        let debug = format!("{output:?}");
        assert!(!debug.contains("safe-out"));
        assert!(!debug.contains("safe-err"));
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn job_creation_and_assignment_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        for (factory, expected) in [
            (
                Arc::new(CreateFailingJobFactory) as Arc<dyn JobFactory>,
                CodexProcessError::JobUnavailable,
            ),
            (
                Arc::new(AssignFailingJobFactory) as Arc<dyn JobFactory>,
                CodexProcessError::JobAssignment,
            ),
        ] {
            let (_sender, cancel) = tokio::sync::watch::channel(false);
            let error = SystemCodexProcessBackend::with_job_factory(factory)
                .run(
                    invocation(directory.path(), "Start-Sleep -Seconds 30"),
                    Duration::from_secs(5),
                    cancel,
                )
                .await
                .unwrap_err();

            assert_eq!(error, expected);
        }
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn output_limit_terminates_the_process_tree() {
        let directory = tempfile::tempdir().unwrap();
        let (_sender, cancel) = tokio::sync::watch::channel(false);
        let error = SystemCodexProcessBackend::default()
            .run(
                invocation(
                    directory.path(),
                    format!(
                        "$bytes = New-Object byte[] {}; [Console]::OpenStandardOutput().Write($bytes, 0, $bytes.Length)",
                        MAX_PROCESS_OUTPUT_BYTES + 1
                    ),
                ),
                PROCESS_TREE_TEST_TIMEOUT,
                cancel,
            )
            .await
            .unwrap_err();

        assert_eq!(error, CodexProcessError::OutputTooLarge);
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn timeout_and_cancellation_stop_hanging_processes() {
        let directory = tempfile::tempdir().unwrap();
        let (_sender, cancel) = tokio::sync::watch::channel(false);
        let started = Instant::now();
        let timeout_error = SystemCodexProcessBackend::default()
            .run(
                invocation(directory.path(), "Start-Sleep -Seconds 30"),
                Duration::from_millis(100),
                cancel,
            )
            .await
            .unwrap_err();
        assert_eq!(timeout_error, CodexProcessError::Timeout);
        assert!(started.elapsed() < Duration::from_secs(5));

        let (sender, cancel) = tokio::sync::watch::channel(false);
        let run = tokio::spawn(async move {
            SystemCodexProcessBackend::default()
                .run(
                    invocation(directory.path(), "Start-Sleep -Seconds 30"),
                    Duration::from_secs(10),
                    cancel,
                )
                .await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        sender.send(true).unwrap();
        assert_eq!(
            run.await.unwrap().unwrap_err(),
            CodexProcessError::Cancelled
        );
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn cancellation_terminates_descendant_processes_in_the_job() {
        let directory = tempfile::tempdir().unwrap();
        let parent_script = directory.path().join("parent.ps1");
        let pid_file = directory.path().join("child.pid");
        let status_file = directory.path().join("parent.status");
        let error_file = directory.path().join("parent.error");
        fs::write(
            &parent_script,
            r#"param([string]$PidFile, [string]$StatusFile, [string]$ErrorFile)
$ErrorActionPreference = 'Stop'
try {
    Set-Content -LiteralPath $StatusFile -Value 'before-start'
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = "$PSHOME\powershell.exe"
    $startInfo.Arguments = '-NoLogo -NoProfile -NonInteractive -Command "Start-Sleep -Seconds 120"'
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $child = [System.Diagnostics.Process]::Start($startInfo)
    Set-Content -LiteralPath $StatusFile -Value 'after-start'
    Set-Content -LiteralPath $PidFile -Value $child.Id
    Set-Content -LiteralPath $StatusFile -Value 'pid-written'
    $child.WaitForExit()
} catch {
    Set-Content -LiteralPath $ErrorFile -Value ($_ | Out-String)
    throw
}
"#,
        )
        .unwrap();
        let (sender, cancel) = tokio::sync::watch::channel(false);
        let run_invocation = CodexInvocation {
            executable: powershell(),
            args: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-File".into(),
                parent_script.as_os_str().to_owned(),
                pid_file.as_os_str().to_owned(),
                status_file.as_os_str().to_owned(),
                error_file.as_os_str().to_owned(),
            ],
            env: Vec::new(),
            workdir: directory.path().to_owned(),
        };
        let run = tokio::spawn(async move {
            SystemCodexProcessBackend::default()
                .run(run_invocation, PROCESS_TREE_TEST_TIMEOUT, cancel)
                .await
        });
        let child_pid = wait_for_process_id(&pid_file, &status_file, &error_file).await;
        sender.send(true).unwrap();

        assert_eq!(
            run.await.unwrap().unwrap_err(),
            CodexProcessError::Cancelled
        );
        assert!(!is_process_running(child_pid));
    }

    struct ChannelSink {
        sender: tokio::sync::mpsc::UnboundedSender<(ProcessStream, Vec<u8>)>,
    }

    impl ProcessEventSink for ChannelSink {
        fn on_output(&self, stream: ProcessStream, bytes: &[u8]) {
            let _ = self.sender.send((stream, bytes.to_vec()));
        }
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn generic_runner_streams_output_before_process_completion() {
        let directory = tempfile::tempdir().unwrap();
        let (event_sender, mut events) = tokio::sync::mpsc::unbounded_channel();
        let sink = Arc::new(ChannelSink {
            sender: event_sender,
        });
        let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
        let run = tokio::spawn(async move {
            SafeProcessRunner::default()
                .run(
                    ProcessInvocation {
                        executable: powershell(),
                        args: vec![
                            "-NoLogo".into(),
                            "-NoProfile".into(),
                            "-NonInteractive".into(),
                            "-Command".into(),
                            "[Console]::Out.Write('first'); [Console]::Out.Flush(); Start-Sleep -Milliseconds 500; [Console]::Out.Write('second')".into(),
                        ],
                        env: filter_inherited_environment(std::env::vars_os()),
                        workdir: directory.path().to_owned(),
                        stdin: None,
                        stdout_file: None,
                    },
                    Duration::from_secs(5),
                    cancel,
                    Some(sink),
                )
                .await
        });

        let (stream, first_chunk) = tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .expect("first output should arrive while the process is running")
            .expect("output channel should remain open");
        assert_eq!(
            stream,
            ProcessStream::Stdout,
            "unexpected first event: {}",
            String::from_utf8_lossy(&first_chunk)
        );
        assert_eq!(first_chunk, b"first");
        assert!(!run.is_finished());

        let output = run.await.unwrap().unwrap();
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout, b"firstsecond");
        assert!(output.stderr.is_empty());
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn generic_runner_writes_structured_stdin_without_debug_exposure() {
        let directory = tempfile::tempdir().unwrap();
        let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
        let invocation = ProcessInvocation {
            executable: powershell(),
            args: vec![
                "-NoLogo".into(),
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                "$value = [Console]::In.ReadToEnd(); [Console]::Out.Write($value.Length)".into(),
            ],
            env: filter_inherited_environment(std::env::vars_os()),
            workdir: directory.path().to_owned(),
            stdin: Some(b"structured-input".to_vec()),
            stdout_file: None,
        };
        let debug = format!("{invocation:?}");
        assert!(!debug.contains("structured-input"));

        let output = SafeProcessRunner::default()
            .run(invocation, Duration::from_secs(5), cancel, None)
            .await
            .unwrap();

        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout, b"16");
        assert!(output.stderr.is_empty());
    }

    #[tokio::test]
    #[serial(codex_process)]
    async fn generic_runner_streams_large_stdout_directly_to_new_file() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("asset.bin");
        let (_cancel_sender, cancel) = tokio::sync::watch::channel(false);
        let output = SafeProcessRunner::default()
            .run(
                ProcessInvocation {
                    executable: powershell(),
                    args: vec![
                        "-NoLogo".into(),
                        "-NoProfile".into(),
                        "-NonInteractive".into(),
                        "-Command".into(),
                        format!(
                            "$bytes = New-Object byte[] {}; [Console]::OpenStandardOutput().Write($bytes, 0, $bytes.Length)",
                            MAX_PROCESS_OUTPUT_BYTES + 1
                        )
                        .into(),
                    ],
                    env: filter_inherited_environment(std::env::vars_os()),
                    workdir: directory.path().to_owned(),
                    stdin: None,
                    stdout_file: Some(destination.clone()),
                },
                PROCESS_TREE_TEST_TIMEOUT,
                cancel,
                None,
            )
            .await
            .unwrap();

        assert_eq!(output.exit_code, Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert_eq!(
            fs::metadata(destination).unwrap().len(),
            (MAX_PROCESS_OUTPUT_BYTES + 1) as u64
        );
    }
}
