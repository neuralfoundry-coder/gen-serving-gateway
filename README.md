# Generative Image Serving Framework

Rust 기반의 생성형 이미지 모델 통합 서빙 프레임워크입니다. 여러 이미지 생성 백엔드를 단일 게이트웨이로 통합하여 관리할 수 있습니다.

## 주요 기능

- **다중 백엔드 통합**: HTTP/gRPC 프로토콜 지원, 다양한 이미지 생성 모델 백엔드 연결
- **게이트웨이 기능**: 부하 분산, 동적 라우팅, API 인증, Rate Limiting
- **비동기 처리**: 비동기 요청 큐, 동적 배치 처리 지원
- **장애 대응**: 헬스 체크, 자동 페일오버, 서킷 브레이커 패턴
- **유연한 응답 형식**: Base64 인코딩, 파일 저장, URL 참조 등 다양한 전송 방식
- **OpenAI API 호환**: 기존 클라이언트와의 호환성 보장

## 아키텍처

```
┌─────────────────────────────────────────────────────────────┐
│                        Clients                               │
│              (HTTP Client, SDK, etc.)                        │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                    Gateway Layer                             │
│  ┌──────────┐  ┌────────────┐  ┌──────────┐  ┌───────────┐ │
│  │ Axum HTTP│  │ API Key    │  │ Rate     │  │ Dynamic   │ │
│  │ Server   │→ │ Auth       │→ │ Limiter  │→ │ Router    │ │
│  └──────────┘  └────────────┘  └──────────┘  └───────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                     Core Layer                               │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────┐  │
│  │ Request Queue│→ │ Batcher      │→ │ Load Balancer     │  │
│  └──────────────┘  └──────────────┘  └─────────┬─────────┘  │
│                                                 │            │
│  ┌──────────────────────────────────────────────▼─────────┐ │
│  │              Health Check Manager                       │ │
│  └────────────────────────────────────────────────────────┘ │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                   Backend Pool                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ SD Backend   │  │ DALL-E       │  │ Custom Model     │   │
│  │ (HTTP)       │  │ (HTTP)       │  │ (gRPC)           │   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## 빠른 시작

### 요구사항

- Rust 1.75 이상
- (선택) protoc (gRPC 기능 사용 시)

### 설치 및 실행

```bash
# 저장소 클론
git clone <repository-url>
cd generative-img-serving

# 빌드
cargo build --release

# 실행
./target/release/img-serving
```

### 설정

`config/default.toml` 파일을 수정하여 설정을 변경할 수 있습니다:

```toml
[server]
host = "0.0.0.0"
port = 8080

[auth]
enabled = true
api_keys = ["your-api-key"]

[rate_limit]
enabled = true
requests_per_second = 100
burst_size = 200

[[backends]]
name = "stable-diffusion"
protocol = "http"
endpoints = ["http://localhost:7860"]
health_check_path = "/health"
health_check_interval_secs = 30
timeout_ms = 60000
weight = 1
enabled = true
```

환경 변수로도 설정할 수 있습니다 (접두사: `IMG_SERVING__`):

```bash
export IMG_SERVING__SERVER__PORT=9090
export IMG_SERVING__AUTH__ENABLED=false
```

## API 엔드포인트

### 이미지 생성 (OpenAI 호환)

```bash
curl -X POST http://localhost:8080/v1/images/generations \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "A beautiful sunset over mountains",
    "n": 1,
    "size": "1024x1024",
    "response_format": "url"
  }'
```

**응답:**
```json
{
  "created": 1234567890,
  "data": [
    {
      "url": "http://localhost:8080/images/abc123.png"
    }
  ]
}
```

### 확장 파라미터

OpenAI API 외에 추가 파라미터를 지원합니다:

```json
{
  "prompt": "...",
  "negative_prompt": "blurry, low quality",
  "seed": 42,
  "guidance_scale": 7.5,
  "num_inference_steps": 50,
  "backend": "stable-diffusion"
}
```

### 백엔드 관리

```bash
# 백엔드 목록 조회
curl http://localhost:8080/v1/backends \
  -H "Authorization: Bearer your-api-key"

# 백엔드 추가
curl -X POST http://localhost:8080/v1/backends \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "new-backend",
    "protocol": "http",
    "endpoints": ["http://gpu-server:7860"]
  }'

# 백엔드 제거
curl -X DELETE http://localhost:8080/v1/backends/backend-name \
  -H "Authorization: Bearer your-api-key"
```

### 헬스 체크

```bash
curl http://localhost:8080/health
```

**응답:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "backends": {
    "total": 2,
    "healthy": 2,
    "unhealthy": 0
  }
}
```

## 로드 밸런싱 전략

- **Round Robin** (기본): 요청을 순차적으로 분배
- **Weighted Round Robin**: 가중치 기반 분배
- **Random**: 무작위 선택
- **Least Connections**: 연결 수 기반 (예정)

## 응답 형식

| 형식 | 설명 |
|------|------|
| `url` | 생성된 이미지의 URL 반환 (기본) |
| `b64_json` | Base64 인코딩된 이미지 데이터 반환 |
| `file` | 로컬 파일 경로 반환 (내부용) |

## 프로젝트 구조

```
src/
├── main.rs                 # 진입점
├── lib.rs                  # 라이브러리 루트
├── config/                 # 설정 관리
├── api/                    # HTTP API
│   ├── routes.rs          # 라우트 정의
│   ├── handlers.rs        # 요청 핸들러
│   └── models.rs          # API 모델
├── middleware/            # 미들웨어
│   ├── auth.rs            # API Key 인증
│   └── rate_limit.rs      # Rate Limiting
├── gateway/               # 게이트웨이
│   ├── load_balancer.rs   # 로드 밸런서
│   ├── health_check.rs    # 헬스 체크
│   └── router.rs          # 동적 라우터
├── queue/                 # 요청 처리
│   ├── request_queue.rs   # 비동기 큐
│   └── batcher.rs         # 배치 처리
├── backend/               # 백엔드 통합
│   ├── traits.rs          # 공통 트레이트
│   ├── http_backend.rs    # HTTP 클라이언트
│   ├── grpc_backend.rs    # gRPC 클라이언트
│   └── registry.rs        # 백엔드 레지스트리
├── response/              # 응답 처리
│   ├── base64.rs          # Base64 인코딩
│   ├── file.rs            # 파일 저장
│   └── url.rs             # URL 생성
└── error.rs               # 에러 처리
```

## 개발

```bash
# 개발 모드 실행
cargo run

# 테스트 실행
cargo test

# 포맷팅
cargo fmt

# 린트 검사
cargo clippy
```

## gRPC 지원

gRPC 백엔드를 사용하려면 `protoc`이 설치되어 있어야 합니다:

```bash
# macOS
brew install protobuf

# Ubuntu
apt-get install protobuf-compiler

# gRPC 코드 생성 포함 빌드
cargo build --features grpc-codegen
```

## Docker 배포

### One-Line 설치 (Ubuntu 24.xx)

서버에서 한 줄 명령어로 즉시 배포할 수 있습니다:

```bash
# Docker Compose 방식 (권장)
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy/quick-install.sh | bash -s compose

# Docker 직접 실행 방식
curl -fsSL https://raw.githubusercontent.com/neuralfoundry-coder/generative-img-serving/main/deploy/quick-install.sh | bash -s docker
```

**옵션:**

```bash
# 포트 변경
curl -fsSL .../quick-install.sh | HOST_PORT=9090 bash -s compose

# 설치 경로 변경
curl -fsSL .../quick-install.sh | INSTALL_DIR=/opt/img-serving bash -s compose

# 특정 버전 설치
curl -fsSL .../quick-install.sh | IMAGE_TAG=0.2.0 bash -s compose
```

### Docker Hub 이미지

```bash
# 최신 버전
docker pull neuralfoundry2coder/generative-img-serving:latest

# 특정 버전
docker pull neuralfoundry2coder/generative-img-serving:0.2.0
```

### 수동 Docker 실행

```bash
docker run -d \
  --name img-serving \
  -p 8080:8080 \
  -v $(pwd)/config:/app/config \
  -v $(pwd)/generated_images:/app/generated_images \
  -e RUST_LOG=info \
  --restart unless-stopped \
  neuralfoundry2coder/generative-img-serving:latest
```

### Docker Compose

```yaml
# docker-compose.yml
services:
  img-serving:
    image: neuralfoundry2coder/generative-img-serving:latest
    ports:
      - "8080:8080"
    volumes:
      - ./config:/app/config:ro
      - ./generated_images:/app/generated_images
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

```bash
docker compose up -d
```

### 로컬 빌드

```bash
# 이미지 빌드
docker build -t generative-img-serving .

# 실행
docker run -d -p 8080:8080 generative-img-serving
```

---

## 릴리스 및 배포 (개발자용)

### 배포 스크립트 (`scripts/deploy.sh`)

통합 배포 스크립트를 통해 Docker Hub에 이미지를 배포할 수 있습니다.

#### 1. 직접 푸시 모드 (Direct)

```bash
# .env 파일 설정 (최초 1회)
cp .env.example .env

# 빌드 및 푸시
./scripts/deploy.sh direct

# 특정 버전으로 푸시
./scripts/deploy.sh direct -v 1.0.0
```

#### 2. 릴리스 모드 (Release via GitHub Actions)

```bash
# Interactive 버전 선택 + 자동 커밋/푸시/태그
./scripts/deploy.sh release

# 버전 선택 화면:
#   [1] Major  : v0.2.0 → v1.0.0  (Breaking changes)
#   [2] Minor  : v0.2.0 → v0.3.0  (New features)
#   [3] Patch  : v0.2.0 → v0.2.1  (Bug fixes)
#   [4] Custom : Enter custom version

# 특정 버전으로 릴리스 (질의 없음)
./scripts/deploy.sh release -v 1.0.0

# 드라이런
./scripts/deploy.sh release -d
```

### GitHub Actions 설정

GitHub Actions를 통한 자동 배포를 위해 저장소에 Secrets 설정이 필요합니다:

1. GitHub 저장소 Settings → Secrets and variables → Actions
2. 다음 시크릿 추가:
   - `DOCKER_USERNAME`: Docker Hub 사용자명
   - `DOCKER_ACCESS_TOKEN`: Docker Hub 액세스 토큰

**자동 트리거:**
- `main` 브랜치 푸시 → 자동 빌드 및 푸시
- 태그 생성 (`v*`) → 버전 태그로 푸시
- Pull Request → 빌드 테스트 (푸시 없음)
- 수동 실행 → Actions 탭에서 workflow_dispatch

## 라이선스

MIT License
