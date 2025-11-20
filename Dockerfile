## 构建阶段
# 使用 RUST  1.91.1 作为基础镜像
FROM rust:1.91.1 AS builder
# 将工作目录切换到 `app` 如果该文件夹不存在 Docker 会创建它
WORKDIR /app
# 安装配置链接器所需的依赖
RUN apt update && apt install lld clang -y
# 将工作环境中的所有文件复制到 Docker 镜像中
COPY . .
ENV SQLX_OFFLINE true
# 使用 release 配置来编译
RUN cargo build --release

## 运行时阶段 
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/zero2prod-axum zero2prod-axum
COPY configuration configuration
ENV APP_ENVIRONMENT production
# 在执行 `docker run` 时,启动该二进制文件
ENTRYPOINT ["./zero2prod-axum"]