# Crab-Vault 🦀

[![Rust](https://img.shields.io/badge/Rust-1.70%2B-dea584?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**分布式对象存储引擎** | 高性能 | 强一致 | 云原生就绪

> 用 Rust 实现的安全、高效的对象存储系统，支持 S3 兼容接口

## 🌟 核心特性
- **跨平台单二进制**：支持 Linux/macOS/Windows 部署
- **对象操作**
- **元数据操作**
- **细粒度 API 访问权限控制**
- **存储引擎**
    - 本地文件系统元数据管理
    - 本地文件系统存储
    - 数据完整性校验 (SHA-256)
- **开发友好**
    - 零配置启动
    - 单二进制部署
    - 详细日志输出

## 🧠 架构概览
```mermaid
graph LR
    Client --> Gateway
    Gateway --> Auth
    Auth --> MetaEngine
    Auth --> DataEngine
    MetaEngine --> KV[分布式KV]
    DataEngine --> Hot[SSD存储层]
    DataEngine --> Cold[EC编码层]
    KV -->|集群模式| TiKV
    KV -->|单机模式| SQLite
```
