<p align="center">
  <img src="assets/cover.svg" alt="Kakao Bridge — 카카오톡과 명령줄을 잇는 연결" width="100%">
</p>

<h1 align="center">Kakao Bridge</h1>

<p align="center">
  <strong>카카오톡을 명령줄과 AI 작업 흐름에 연결합니다.</strong><br>
  실행 중인 Windows 카카오톡에서 대화와 파일을 다루는 실험용 CLI입니다.
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-x64-1479e8" alt="Windows x64">
  <img src="https://img.shields.io/badge/상태-실험용-f0b72f" alt="실험용">
  <a href="https://github.com/contentriumkorea/Kakao-Bridge/releases/latest"><img src="https://img.shields.io/badge/배포-다운로드-222222" alt="최신 배포"></a>
</p>

<p align="center">
  <a href="#빠르게-시작하기">빠르게 시작하기</a> ·
  <a href="https://github.com/contentriumkorea/Kakao-Bridge/releases/latest">다운로드</a> ·
  <a href="#사용-모습">사용 모습</a> ·
  <a href="#ai-에이전트에서-사용하기">AI 스킬</a> ·
  <a href="#적용-범위와-제한">적용 범위와 제한</a>
</p>

---

## 빠르게 시작하기

Windows x64, 로그인된 PC 카카오톡, Microsoft Visual C++ 런타임이 필요합니다. 파일 다운로드에는 Node.js 20 이상이 추가로 필요합니다.

1. [최신 릴리스](https://github.com/contentriumkorea/Kakao-Bridge/releases/latest)에서 ZIP을 내려받고 압축을 풉니다.
2. 압축을 푼 폴더에서 PowerShell을 열고 다음 명령을 실행합니다.

```powershell
.\kakao.cmd status
.\kakao.cmd chats
```

설치 프로그램 없이 실행하며, 압축 안의 폴더 구조를 유지해야 합니다.

## 필요한 작업을 한 흐름으로

| 01 · 대화 찾기 | 02 · 메시지 다루기 | 03 · 파일 주고받기 |
| :--- | :--- | :--- |
| **목록을 확인합니다** | **기록을 읽고 보냅니다** | **첨부파일을 전송·저장합니다** |
| 로컬 친구 후보와 대화 목록을 조회합니다. | 대화 기록을 읽고 열린 창에 메시지를 보냅니다. | 열린 채팅창에 파일을 보내고 가능한 첨부를 내려받습니다. |

## 사용 모습

<p align="center">
  <img src="assets/flow.svg" alt="명령줄에서 카카오톡을 거쳐 JSON 결과를 받는 흐름" width="880">
</p>

**명령은 JSON 결과로 돌아옵니다.** 이미 로그인된 PC 카카오톡을 이용합니다.

<details>
<summary><strong>추가 명령 예시</strong></summary>

```powershell
.\kakao.cmd friends
.\kakao.cmd open-chats
'{"chat_id":"대화방 ID"}' | .\kakao.cmd messages
```

</details>

## 배포 방식

| 방식 | 사용 방법 |
| :--- | :--- |
| Windows ZIP | 압축을 풀고 포함된 `kakao.cmd`를 실행합니다. |
| 다른 폴더에서 실행 | `kakao.cmd`의 전체 경로를 지정합니다. |
| Codex / Claude Code 스킬 | ZIP 또는 저장소의 `skills/kakao-bridge`를 각 에이전트의 스킬 폴더에 복사합니다. |

## AI 에이전트에서 사용하기

전용 스킬은 필수는 아니지만 명령 형식과 전송 시 주의사항을 에이전트에 알려 줍니다. 압축을 푼 폴더에서 다음 명령으로 각각 설치할 수 있습니다. 같은 이름의 스킬이 이미 있으면 덮어쓰지 않고 중단합니다.

```powershell
$dirs = @("$env:USERPROFILE\.codex\skills", "$env:USERPROFILE\.claude\skills")
foreach ($dir in $dirs) {
  if (Test-Path -LiteralPath (Join-Path $dir 'kakao-bridge')) { throw "스킬이 이미 존재합니다: $dir" }
}
foreach ($dir in $dirs) {
  New-Item -ItemType Directory -Path $dir -Force | Out-Null
  Copy-Item -LiteralPath '.\skills\kakao-bridge' -Destination (Join-Path $dir 'kakao-bridge') -Recurse
}
```

에이전트가 실행 파일을 찾지 못하면 압축을 푼 `kakao.cmd`의 전체 경로를 알려 주세요. 에이전트를 시작하는 환경의 `KAKAO_BRIDGE_CMD`에 전체 경로를 지정하거나 `PATH`에서 찾을 수 있도록 설정해도 됩니다. 스킬을 복사해도 프로그램이나 환경변수가 자동으로 설치되지는 않습니다.

## 이렇게 사용하세요

```powershell
# chats에서 확인한 ID로 채팅창 열기
'{"chat_id":"대화방 ID"}' | .\kakao.cmd open

# 열린 채팅창에 파일 보내기
@{ chat='열린 채팅창 이름'; file='C:\절대경로\파일.pdf' } |
  ConvertTo-Json -Compress | .\kakao.cmd send-file
```

## 적용 범위와 제한

- **실험용 프로그램**입니다. 현재 카카오톡 버전과 창 상태에 따라 동작이 달라질 수 있습니다.
- 배포된 실행 파일과 라이브러리는 코드 서명이 없습니다. 함께 제공하는 SHA-256은 파일의 일치 여부를 확인하기 위한 값이며 서명을 대신하지 않습니다.
- 채팅창 열기는 현재 96 DPI에서만 시도합니다. 검색 결과의 첫 행을 선택하므로 다른 방이 열릴 수 있습니다. 제목이 다르면 오류로 중단하고 자동 전송하지 않습니다. 제목 일치만으로 방 ID가 검증되지는 않습니다.
- 파일 전송 중 다른 창의 포커스가 바뀌고 마우스 커서가 잠시 이동할 수 있습니다. `submitted_unconfirmed`는 상대방 수신을 확인했다는 뜻이 아닙니다. 불확실한 경우 자동 재전송하지 말고 대화 기록을 확인하세요.
- 나와의 채팅에서 작은 텍스트 파일과 PNG를 보낸 뒤 다시 내려받아 내용 일치를 확인했습니다. 모든 파일 형식이 검증된 것은 아닙니다. 시험한 `.bin` 파일은 카카오톡에서 거절됐습니다.
- 로컬 캐시의 목록과 메시지만 반환합니다. 친구 목록은 추정된 후보이며 완전한 서버 목록을 뜻하지 않습니다.

<details>
<summary><strong>개인정보와 동작 방식</strong></summary>

프로그램은 실행 중인 카카오톡 프로세스의 메모리에서 로컬 데이터베이스를 읽는 데 필요한 키를 찾아 사용합니다. 키와 서명된 파일 주소를 일반 출력이나 파일에 기록하지 않도록 설계했지만, 운영체제의 크래시 덤프까지 통제하지는 못합니다. CLI 결과에는 개인 대화 내용이 포함될 수 있으므로 출력과 로그를 신중하게 다루세요. 파일 전송은 카카오톡 창을 조작하며 클립보드는 사용하지 않습니다.

</details>

<details>
<summary><strong>English overview</strong></summary>

Kakao Bridge is an experimental Windows command-line interface for an already signed-in KakaoTalk PC client. Room opening and file sending may change focus, and file sending may briefly move the pointer. See the limitations above before use.

</details>

---

<p align="center">
  <strong>Contentrium</strong><br>
  <a href="https://github.com/contentriumkorea/Kakao-Bridge/releases/latest">Latest release</a> ·
  <a href="THIRD-PARTY.md">제3자 라이선스</a>
</p>

<sub>Kakao Bridge는 카카오의 공식 제품이 아닌 독립 프로젝트입니다. 제3자 구성요소의 라이선스는 배포 파일에 포함되어 있습니다.</sub>

<p align="center">
  <a href="https://contentrium.com/"><img src="assets/home.svg" alt="콘텐츠리움 홈페이지 바로가기" width="260" height="52"></a>
</p>
