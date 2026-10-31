---
name: kakao-bridge
description: Use when controlling an already signed-in Windows KakaoTalk PC client with the Kakao Bridge public preview CLI.
---

# Kakao Bridge

This skill is for the **public preview**. Resolve `kakao.cmd` from `KAKAO_BRIDGE_CMD` or `PATH`, then run `status` and verify `edition` is `public_preview` before using these commands. If no matching executable is configured, ask for its location. The separately installed personal edition has a different command contract.

```powershell
$kb = $env:KAKAO_BRIDGE_CMD
if (-not $kb) { $found = Get-Command kakao.cmd -ErrorAction SilentlyContinue; if ($found) { $kb = $found.Source } }
if (-not $kb) { throw 'Kakao Bridge public preview path is not configured' }
$status = & $kb status | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $status.edition -ne 'public_preview') { throw 'This is not the Kakao Bridge public preview' }
if (-not $status.kakao_running) { throw 'KakaoTalk PC is not running' }
```

Commands return JSON. `status` checks the app; `open-chats` lists titles of currently open chat windows; `screen` reads the visible main window using OCR. Send `{"chat":"exact open window title"}` to `messages` on standard input to OCR the rendered part of an open chat. OCR may be incomplete or inaccurate, does not prove recipient identity, and cannot read minimized windows. Outputs can contain private conversations and may be transmitted to the AI service processing them.

Only send a message or file when the user has requested it. Both `send` and `send-file` may activate KakaoTalk or move focus; a message to the active chat may first activate the main KakaoTalk window. Use them only within the user's existing authorization for that effect. The title must exactly match one uniquely open chat window, but title matching does not prove the room ID or recipient identity. Use title-only identification only within the user's existing authorization for that limitation; OCR or an open-window list alone does not establish identity. If focus change is disallowed or the target is ambiguous, stop rather than bypassing the guards.

Pass JSON through standard input. `send` takes `chat`, a nonempty single-line `message` of at most 16,000 UTF-8 bytes with no control characters (including tabs), `allow_focus_change: true`, and `allow_unverified_title: true`. It also accepts `allow_placeholder: true` only to replace a recognized empty-field hint; do not clear a draft. `send-file` takes `chat`, an absolute path to an existing nonempty regular `file`, and the same two explicit boolean flags. It may also move the mouse briefly. For example:

```powershell
@{chat='exact open window title';message='requested message';allow_focus_change=$true;allow_unverified_title=$true} |
  ConvertTo-Json -Compress | & $kb send
```

`submitted_unconfirmed` means only that submission was requested, not that the recipient received it. For `SUBMISSION_UNKNOWN`, `DRAFT_PREPARED_UNSENT`, or any uncertain outcome, do not retry automatically; inspect the state first. Preserve existing drafts and any dialog left open after an error. Do not claim an actual file or message was delivered without verification.

The preview does not support full friend/chat lists, opening a room, or downloading attachments. Do not invent those commands or substitute the private edition without the user's authorization.
