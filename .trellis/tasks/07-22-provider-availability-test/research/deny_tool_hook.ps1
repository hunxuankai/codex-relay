$null = [Console]::In.ReadToEnd()
@{
  hookSpecificOutput = @{
    hookEventName = 'PreToolUse'
    permissionDecision = 'deny'
    permissionDecisionReason = 'Codex Relay probe blocks all local tools.'
  }
} | ConvertTo-Json -Compress
