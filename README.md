<p align="center">
  <img src="assets/cover-kakao-bridge.svg" alt="Kakao Bridge — PC 카카오톡과 AI 작업 흐름을 연결합니다" width="100%">
</p>

<h1 align="center">Kakao Bridge</h1>

<p align="center">
  <strong>PC 카카오톡을 AI와 명령줄에 연결하세요.</strong><br>
  이미 로그인된 카카오톡으로 확인하고, 읽고, 전송하는 Windows용 도구입니다.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-x64-286789?style=flat-square" alt="Windows x64">
  <img src="https://img.shields.io/badge/버전-v0.2.0--alpha.4-e8ba39?style=flat-square" alt="버전 v0.2.0-alpha.4">
</p>

<p align="center">
  <a href="https://github.com/contentriumkorea/Kakao-Bridge/releases/tag/v0.2.0-alpha.4"><img src="assets/download-button-ko.svg" alt="Kakao Bridge 다운로드" width="330"></a>
</p>

<p align="center">
  <a href="#빠르게-시작하기">빠르게 시작하기</a> ·
  <a href="#주요-기능">주요 기능</a> ·
  <a href="#ai-에이전트에서-사용하기">AI 스킬</a> ·
  <a href="#제한과-개인정보">적용 범위와 제한</a>
</p>

---

## 빠르게 시작하기

**Windows x64**, 로그인된 PC 카카오톡, Windows 한글 OCR이 필요합니다.

1. [Kakao Bridge ZIP](https://github.com/contentriumkorea/Kakao-Bridge/releases/tag/v0.2.0-alpha.4)을 내려받아 압축을 풉니다.
2. 압축을 푼 폴더에서 PowerShell을 열고 아래 명령을 실행합니다.

```powershell
.\kakao.cmd status       # 카카오톡 실행 상태
.\kakao.cmd open-chats   # 현재 열린 채팅창 제목
.\kakao.cmd screen       # 카카오톡 메인 창 OCR
```

결과는 JSON으로 반환됩니다. `status`에는 `edition: public_preview`가 표시됩니다.

## 주요 기능

확인하고, 읽고, 보내는 작업을 하나의 흐름으로 연결합니다.

| 01 · 확인 | 02 · 화면 읽기 | 03 · 전송 |
| :--- | :--- | :--- |
| **대화창을 열고 확인합니다** | **화면에 표시된 부분을 읽습니다** | **메시지와 파일을 보냅니다** |
| 닫힌 메인 창을 복원하고 대화방을 찾아 열거나 열린 창 제목을 확인합니다. | 메인 창이나 이미 열린 대화창을 OCR로 읽습니다. | 이미 열린 대화창에 한 줄 메시지나 파일을 전송합니다. |

## 사용 모습

<p align="center">
  <img src="assets/flow-showcase.svg" alt="명령줄 또는 AI 에이전트에서 카카오톡 PC로 요청하고 JSON 결과를 받는 흐름" width="100%">
</p>

**명령 한 번으로 요청하고, JSON으로 결과를 확인합니다.** 이미 로그인된 PC 카카오톡을 사용하므로 별도 로그인 과정이 없습니다.

<details>
<summary><strong>열린 채팅창 화면 읽기 예시</strong></summary>

```powershell
@{chat='정확한 열린 창 제목'} | ConvertTo-Json -Compress | .\kakao.cmd messages
```

화면에 렌더링된 부분을 OCR로 읽으므로 일부 내용이 빠지거나 잘못 인식될 수 있습니다. 창 제목만으로 대화방의 실제 ID나 수신자가 확인되는 것은 아닙니다.

</details>

## AI 에이전트에서 사용하기

Codex와 Claude Code에서 사용할 **Kakao Bridge 스킬**이 ZIP의 `skills/kakao-bridge`에 포함되어 있습니다. 각 에이전트의 스킬 폴더에 복사하고, 압축을 푼 `kakao.cmd`의 전체 경로를 알려 주세요. 같은 이름의 스킬이 이미 있다면 기존 설치본을 덮어쓰지 마세요.

| 구성 | 사용 방법 |
| :--- | :--- |
| Windows ZIP | 압축을 풀고 `kakao.cmd`를 실행합니다. |
| AI 스킬 | `skills/kakao-bridge`를 Codex 또는 Claude Code의 스킬 폴더에 복사합니다. |

## 전송하기

`open`으로 대상 대화창을 열 수 있습니다. 현재 대화방 열기는 96 DPI에서만 시도합니다. 창을 열거나 전송하는 중 포커스가 바뀔 수 있으며, 파일 전송은 마우스 포인터도 잠시 움직일 수 있습니다. 열린 창 제목이 정확히 일치해도 실제 방 ID나 수신자를 증명하지는 않습니다. 이 두 조건을 허용할 때 아래 옵션을 지정합니다.

```powershell
# 나와의 채팅 열기: 본인 창의 정확한 제목을 지정합니다.
@{chat='본인 창의 정확한 제목';self=$true;allow_focus_change=$true;allow_unverified_title=$true} |
  ConvertTo-Json -Compress | .\kakao.cmd open

@{chat='정확한 열린 창 제목';message='한 줄 메시지';allow_focus_change=$true;allow_unverified_title=$true} |
  ConvertTo-Json -Compress | .\kakao.cmd send

@{chat='정확한 열린 창 제목';file='C:\절대경로\파일.pdf';allow_focus_change=$true;allow_unverified_title=$true} |
  ConvertTo-Json -Compress | .\kakao.cmd send-file
```

`submitted_unconfirmed`는 전송 요청을 뜻하며 수신 확인은 아닙니다. 결과가 불확실하면 자동으로 다시 보내지 말고 대화창을 확인하세요. 입력창에 작성 중인 내용이 있으면 메시지 전송을 중단합니다.

입력란의 기존 내용은 안내 문구와 같아도 초안일 수 있습니다. 해당 내용을 대체하도록 직접 허용할 때만 `allow_placeholder: true`와 정확히 일치하는 `expected_existing_text`를 함께 지정하세요. 메시지 전송 중에는 같은 채팅창과 입력란을 사용하지 마세요. 전송 확인과 사용자 입력은 완전히 원자적으로 처리되지 않습니다.

열려 있지 않은 다른 대화방을 검색해 열 때는 `allow_search_input_changes: true`가 추가로 필요합니다. 검색 중 같은 검색창을 조작하면 입력이 바뀔 수 있으므로 이 영향을 허용한 경우에만 사용하세요. 검색어는 자동으로 지우지 않습니다. 나와의 채팅이나 이미 열린 창에는 이 옵션이 필요하지 않습니다.

## 제한과 개인정보

- 현재 공개판은 **전체 친구·대화 목록 조회와 파일 다운로드를 지원하지 않습니다.**
- OCR은 화면에 표시된 일부만 읽으며 누락이나 오인식이 있을 수 있습니다. 최소화된 창은 읽을 수 없습니다.
- 이 공개판은 복호화 키를 탐색하거나 로컬 대화 데이터베이스를 직접 복호화하지 않습니다.
- 명령 결과에는 이름과 대화 내용이 포함될 수 있습니다. AI 서비스에 전달하면 해당 서비스로 내용이 전달될 수 있습니다.
- 실행 파일은 코드 서명이 없습니다. 동봉된 `SHA256SUMS.txt`로 다운로드한 파일의 일치 여부를 확인할 수 있습니다.

<details>
<summary><strong>시험 범위</strong></summary>

나와의 채팅에서 메시지와 파일 전송을 확인했습니다. 파일은 별도 개인용 도구로 다시 내려받아 원본과 해시가 일치하는 것을 확인했습니다. 공개판 자체는 파일 다운로드를 지원하지 않습니다. 전송 결과가 불확실하게 표시된 사례도 있어, 그 경우 실제 대화 기록을 확인한 뒤 진행해야 합니다.

</details>

<details>
<summary><strong>English overview</strong></summary>

Kakao Bridge connects an already signed-in KakaoTalk PC client with command-line and AI workflows. It can inspect open window titles, read visible text with OCR, open a room by title, and request message or file sends to that room. Focus may change, OCR can be incomplete, and window titles do not prove recipient identity.

</details>

---

<p align="center">
  <strong>Contentrium</strong><br>
  <a href="https://github.com/contentriumkorea/Kakao-Bridge/releases/tag/v0.2.0-alpha.4">Kakao Bridge 다운로드</a> ·
  <a href="THIRD-PARTY.md">제3자 라이선스</a>
</p>

<sub>Kakao Bridge는 카카오의 공식 제품이 아닌 독립 프로젝트입니다.</sub>

<p align="center">
  <a href="https://contentrium.com/"><img src="assets/home.svg" alt="콘텐츠리움 홈페이지 바로가기" width="260" height="52"></a>
</p>

