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

Commands return JSON. `capabilities` reports supported commands and limits. `status` checks the app; `open-chats` lists titles of currently open chat windows; `screen` reads the visible main window using OCR. `friends` and `chats` OCR only the currently visible portion of the respective main-window list. They return OCR lines, not verified names or a complete account list: search filters, scrolling, truncation, and OCR errors can omit or change entries. `LIST_TAB_NOT_VISIBLE` means the corresponding tab is not currently displayed. Send `{"chat":"exact open window title"}` to `messages` on standard input to OCR the rendered part of an open chat. OCR may be incomplete or inaccurate, does not prove recipient identity, and cannot read minimized windows. Outputs can contain private conversations and may be transmitted to the AI service processing them.

To select a list tab, `show-list` accepts `list: "friends"` or `"chats"` plus `allow_focus_change: true` and `allow_search_input_changes: true`. Switching can affect concurrent search input. Only set the latter if the user explicitly accepts that risk; never infer it merely from focus permission. If the search contains text or a search is active, preserve it and stop. Afterwards call `friends` or `chats`; the result remains only visible OCR, not a complete list.

To restore the running client's hidden or minimized main window, pass `{"allow_focus_change":true}` to `show-main`. To open an existing room, pass `{"chat":"exact expected title","allow_focus_change":true,"allow_unverified_title":true}` to `open`; for the user's own chat add `"self":true` and use the exact expected display title. Opening can change focus, scroll position, and read state; it currently operates only at 96 DPI. It searches by title and can select the first result. An `opened_title_match` response verifies only the window title, not the internal room ID or recipient. A duplicate title or ambiguous recipient requires stopping rather than sending. Existing search text is preserved; do not clear it to force opening. For an unopened non-self room, `allow_search_input_changes: true` is required; do not infer that permission from focus authorization. Use it only if the user explicitly accepts possible changes to concurrent search input and the search box is not in use. The query is left in the search field. Respect the user's existing authorization for focus and title-only identification.

Only send a message or file when the user has requested it. Both `send` and `send-file` may activate KakaoTalk or move focus; a message to the active chat may first activate the main KakaoTalk window. Use them only within the user's existing authorization for that effect. The title must exactly match one uniquely open chat window, but title matching does not prove the room ID or recipient identity. Use title-only identification only within the user's existing authorization for that limitation; OCR or an open-window list alone does not establish identity. If focus change is disallowed or the target is ambiguous, stop rather than bypassing the guards.

## Requests to send to the user

An explicit request such as “나한테 보내” authorizes sending the specified content to the user's own chat. Do not ask the user to confirm the same requested file, recipient, focus change, or title-only limitation again when their prior authorization and the intended self chat are already established. `self-title` retrieves an optional locally user-bound self-chat title; it is protected for the current Windows user, but it is still a title, not proof of the logged-in account or underlying chat ID. With the user's existing authorization for title-only identification and focus changes, use `self-title`, verify the KakaoTalk process and unique title, use `open` with `self: true` when needed, and send the exact requested item. Never substitute a similarly named room or retry an uncertain submission. If the account may have changed, the binding is missing, the title is ambiguous, or identity cannot be established within the user's authorized constraints, stop and resolve that actual missing information rather than inventing it.

For one-time setup, `bind-self` accepts `chat` with the exact self-chat title and `acknowledge_title_only: true`; only bind after the user has authorized that limitation and the intended self chat has been independently established. It will not overwrite an existing binding. The binding is local to the current Windows user and is not included in the distribution.

Pass JSON through standard input. `send` takes `chat`, a nonempty single-line `message` of at most 16,000 UTF-8 bytes with no control characters (including tabs), `allow_focus_change: true`, and `allow_unverified_title: true`. `send-file` takes `chat`, an absolute path to an existing nonempty regular `file`, and the same two explicit boolean flags. It may also move the mouse briefly. For example:

```powershell
@{chat='exact open window title';message='requested message';allow_focus_change=$true;allow_unverified_title=$true} |
  ConvertTo-Json -Compress | & $kb send
```

`submitted_unconfirmed` means only that submission was requested, not that the recipient received it. For `SUBMISSION_UNKNOWN`, `DRAFT_PREPARED_UNSENT`, or any uncertain outcome, do not retry automatically; inspect the state first. Preserve existing drafts and any dialog left open after an error. Do not claim an actual file or message was delivered without verification.

File sending activates the verified chat and uses KakaoTalk's Ctrl+T shortcut. Before opening the file chooser, Korean OCR availability is checked. `KOREAN_OCR_UNAVAILABLE` or `OCR_PREFLIGHT_UNAVAILABLE` means no shortcut was sent; use `doctor` to diagnose and report the missing Windows Korean OCR capability. Do not automatically install or change Windows language settings. The confirmation button is identified with Korean OCR; ambiguous or unavailable confirmation remains uncertain and must not be retried blindly. GUI operations from either edition share a session lock; `AUTOMATION_BUSY` means another operation is running. `doctor` checks environment and OCR without reading chats. Do not send while the user is actively typing or using the mouse. The CLI may retry a foreground activation only before injecting a shortcut, while checking for input or target changes. `INPUT_BUSY` or `FOCUS_SWITCH_FAILED` means the shortcut was not sent; do not force focus or bypass the guard. For `SHORTCUT_INPUT_UNKNOWN`, `FILE_DIALOG_NOT_FOUND`, `INPUT_INTERRUPTED_DIALOG_OPEN`, or any uncertain state, do not automatically retry or dismiss a dialog; inspect the current state first. OS focus and input checks cannot be atomic with a simultaneous user action.

Never assume text matching an input hint is disposable: a user could have typed that exact text as a draft. If `DRAFT_PRESENT` or `EXISTING_TEXT_CONFIRMATION_REQUIRED` occurs, preserve it. Only if the user explicitly authorizes replacing the exact existing text may `allow_placeholder: true` and `expected_existing_text` with that exact text be supplied together. Do not infer permission from a hint-like value. Do not operate the same chat or input during message submission: the checks are not atomic with user interaction and do not establish recipient delivery.

The public edition does not support complete friend/chat lists or downloading attachments. Do not claim OCR output is complete, invent a download command, or substitute the private edition without the user's authorization.

Window navigation uses the target window DPI context; DPI-aware KakaoTalk configurations beyond 96 DPI have only been exercised with synthetic Windows fixtures, so verify the room and never send on ambiguous results.
