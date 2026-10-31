---
name: kakao-bridge
description: Use when controlling an already signed-in Windows KakaoTalk PC client with the Kakao Bridge public CLI.
---

# Kakao Bridge

This skill is for the **public edition**. Resolve `kakao.cmd` from `KAKAO_BRIDGE_CMD` or `PATH`, then run `status` and verify `edition` is `public_preview` before using these commands. If no matching executable is configured, ask for its location. The separately installed personal edition has a different command contract.

```powershell
$kb = $env:KAKAO_BRIDGE_CMD
if (-not $kb) { $found = Get-Command kakao.cmd -ErrorAction SilentlyContinue; if ($found) { $kb = $found.Source } }
if (-not $kb) { throw 'Kakao Bridge public preview path is not configured' }
$status = & $kb status | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $status.edition -ne 'public_preview') { throw 'This is not the Kakao Bridge public preview' }
if (-not $status.kakao_running) { throw 'KakaoTalk PC is not running' }
```

Commands return JSON. `status` checks the app; `open-chats` lists titles of currently open chat windows; `screen` reads the visible main window using OCR. Send `{"chat":"exact open window title"}` to `messages` on standard input to OCR the rendered part of an open chat. OCR may be incomplete or inaccurate, does not prove recipient identity, and cannot read minimized windows. Outputs can contain private conversations and may be transmitted to the AI service processing them.

To restore the running client's hidden or minimized main window, pass `{"allow_focus_change":true}` to `show-main`. To open an existing room, pass `{"chat":"exact expected title","allow_focus_change":true,"allow_unverified_title":true}` to `open`; for the user's own chat add `"self":true` and use the exact expected display title. Opening can change focus, scroll position, and read state; it currently operates only at 96 DPI. It searches by title and can select the first result. An `opened_title_match` response verifies only the window title, not the internal room ID or recipient. A duplicate title or ambiguous recipient requires stopping rather than sending. Existing search text is preserved; do not clear it to force opening. For an unopened non-self room, `allow_search_input_changes: true` is required; do not infer that permission from focus authorization. Use it only if the user explicitly accepts possible changes to concurrent search input and the search box is not in use. The query is left in the search field. Respect the user's existing authorization for focus and title-only identification.

Only send a message or file when the user has requested it. Both `send` and `send-file` may activate KakaoTalk or move focus; a message to the active chat may first activate the main KakaoTalk window. Use them only within the user's existing authorization for that effect. The title must exactly match one uniquely open chat window, but title matching does not prove the room ID or recipient identity. Use title-only identification only within the user's existing authorization for that limitation; OCR or an open-window list alone does not establish identity. If focus change is disallowed or the target is ambiguous, stop rather than bypassing the guards.

Pass JSON through standard input. `send` takes `chat`, a nonempty single-line `message` of at most 16,000 UTF-8 bytes with no control characters (including tabs), `allow_focus_change: true`, and `allow_unverified_title: true`. `send-file` takes `chat`, an absolute path to an existing nonempty regular `file`, and the same two explicit boolean flags. It may also move the mouse briefly. For example:

```powershell
@{chat='exact open window title';message='requested message';allow_focus_change=$true;allow_unverified_title=$true} |
  ConvertTo-Json -Compress | & $kb send
```

`submitted_unconfirmed` means only that submission was requested, not that the recipient received it. For `SUBMISSION_UNKNOWN`, `DRAFT_PREPARED_UNSENT`, or any uncertain outcome, do not retry automatically; inspect the state first. Preserve existing drafts and any dialog left open after an error. Do not claim an actual file or message was delivered without verification.

Never assume text matching an input hint is disposable: a user could have typed that exact text as a draft. If `DRAFT_PRESENT` or `EXISTING_TEXT_CONFIRMATION_REQUIRED` occurs, preserve it. Only if the user explicitly authorizes replacing the exact existing text may `allow_placeholder: true` and `expected_existing_text` with that exact text be supplied together. Do not infer permission from a hint-like value. Do not operate the same chat or input during message submission: the checks are not atomic with user interaction and do not establish recipient delivery.

The public edition does not support complete friend/chat lists or downloading attachments. Do not invent those commands or substitute the private edition without the user's authorization.

