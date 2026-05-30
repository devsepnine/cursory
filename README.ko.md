# Cursory

[English](README.md)

> Windows에서 마우스 커서를 창·모니터·사용자 지정 사각형 안에 가두는 유틸리티.

![license](https://img.shields.io/badge/license-MIT-blue)
![platform](https://img.shields.io/badge/platform-Windows-lightgrey)

<!-- TODO: 데모 GIF 추가 (창 + 커서가 보이지 않는 벽에 막히는 장면 + 해제), 예: assets/demo.gif -->
<!-- ![Cursory demo](assets/demo.gif) -->

## 기능

- **3가지 가두기 모드** — 앱 창, 모니터, 화면에 직접 그린 사용자 지정 사각형 중
  하나에 커서를 가둡니다.
- **전역 핫키** — 어디서나 토글. 새 조합은 앱 안에서 녹화·미리보기·확정합니다
  (기본 `Ctrl+Alt+L`).
- **시스템 트레이** — 트레이로 최소화하고 클릭으로 복원. 닫기 버튼이 앱을 종료할지
  트레이로 숨길지 선택할 수 있습니다.
- **부팅 시 자동 실행** — 사용자별 레지스트리 Run 키 기반 (선택).
- **단일 인스턴스** — 두 번째 실행 시 중복 창 대신 기존 창을 앞으로 가져옵니다.
- **실시간 반영** — 해상도나 모니터 구성이 바뀌면 자동으로 다시 적용합니다.
- **패딩** — 가두는 영역을 지정한 만큼 안쪽으로 줄일 수 있습니다.

## 설치

[Releases](https://github.com/devsepnine/cursory/releases) 페이지에서 최신
빌드를 받으세요:

- **`Cursory-x.y.z.msi`** — 설치 프로그램. 시작 메뉴에 등록됩니다("Cursory" 검색).
- **`Cursory-x.y.z.exe`** — 포터블. 설치 없이 바로 실행.

> 바이너리에 코드 서명이 없어 첫 실행 시 Windows SmartScreen 경고가 뜰 수 있습니다.
> **추가 정보 → 실행**을 선택하세요.

## 사용법

1. **모드** 선택: 앱 창 / 모니터 / 사용자 지정 사각형.
2. 대상 선택 — 목록에서 창을 고르거나, 모니터를 고르거나, 사각형을 그립니다.
3. **ACTIVATE**(또는 전역 핫키)를 눌러 커서를 가두고, 다시 누르면 해제됩니다.

- **핫키** — 설정에서 *Change* 클릭 → 조합 입력 → *Confirm*.
- **닫기 버튼** — 트레이로 보내기 / 앱 종료 중 선택.
- **부팅 시 자동 실행** / **활성화 시 최소화** — 설정에서 토글.

설정은 `%APPDATA%\cursory\settings.conf`에 저장됩니다.

## 소스에서 빌드

Rust 툴체인이 필요합니다 (1.85+, edition 2024).

```powershell
cargo build --release
# -> target/release/cursory.exe
```

### MSI 패키징

[WiX Toolset v3.14](https://github.com/wixtoolset/wix3/releases)와 `cargo-wix`가
필요합니다 (스크립트가 `cargo-wix`는 자동 설치).

```powershell
pwsh scripts/release.ps1
# -> dist/Cursory-<version>.msi, dist/Cursory-<version>.exe
```

## 라이선스

[MIT](LICENSE) © HibiCanvas
