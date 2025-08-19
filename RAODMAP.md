# 🗺 crab-vault 发展路线图

## 架构演进
```mermaid
graph LR
    A[v0.1 单机原型] --> B[v0.3 元数据持久化]
    B --> C[v0.5 分布式基础]
    C --> D[v1.0 生产就绪]
```

## 详细里程碑

### 🚀 v0.1 - 单机原型 (当前)
**核心目标**: MVP基础功能
```mermaid
graph TB
    Client[HTTP客户端] --> API[API接口层]
    API --> Meta[内存元数据]
    API --> Storage[本地文件存储]
```
- [x] RESTful API端点 (GET/PUT/DELETE)
- [x] 内存元数据管理
- [x] 本地文件系统存储
- [x] SHA-256数据校验
- [x] 错误处理
- [x] 桶(Bucket)概念引入
- [x] 配置文件支持 (TOML格式)

### 🔒 v0.2 - 元数据持久化（文件系统）
**架构升级**:
```mermaid
graph TB
    API[API层] --> MetaDB[元数据引擎]
    MetaDB --> FSMeta[文件系统]
    Storage[存储层] --> FS[文件系统]
```
- [x] 元数据引擎抽象层
- [x] 文件系统持久化存储
- [ ] 文件上传时自动计算元数据
- [ ] 分块上传支持

### 🔒 v0.3 - 元数据持久化（嵌入式 SQLite）
**架构升级**:
```mermaid
graph TB
    API[API层] --> MetaDB[元数据引擎]
    MetaDB --> SQLite[SQLite]
    MetaDB --> Raft[Raft集群]
    Storage[存储层] --> FS[文件系统]
```
- [ ] 元数据缓存层 (LRU缓存)
- [ ] 元数据备份/恢复机制
- [ ] SQLite持久化存储
- [ ] 分块上传支持
- [ ] 基准测试套件

### 🌐 v0.5 - 分布式基础
**架构升级**:
```mermaid
graph LR
    Client --> LB[负载均衡]
    LB --> Node1[节点]
    LB --> Node2[节点]
    subgraph 节点
        API[API层] --> Meta[元数据]
        API --> Storage[存储层]
        Meta --> Raft[Raft共识]
    end
    Raft -.->|跨节点| Raft集群
    Storage -.->|数据复制| 其他节点
```
- [ ] 引入 openraft
- [ ] 数据分片策略 (一致性哈希)
- [ ] 节点健康检查
- [ ] 数据迁移工具
- [ ] 节点自动发现
- [ ] 数据复制机制
- [ ] S3 API兼容层
- [ ] 监控指标暴露 (Prometheus)

### 🏭 v1.0 - 生产就绪
**架构升级**:
```mermaid
graph LR
    Client --> Gateway[API网关]
    Gateway --> Meta[分布式元数据]
    Gateway --> Storage[多存储后端]
    Storage --> FS[文件系统]
    Storage --> EC[纠删码存储]
    Storage --> Cloud[云存储]
```
- [ ] 纠删码存储支持
- [ ] 数据冷热分层
- [ ] 服务端加密
- [ ] 权限控制系统
- [ ] 生命周期管理
- [ ] 配额管理 (存储空间/请求速率)
- [ ] 多租户隔离
- [ ] 审计日志

### 🔮 未来方向
- 全局命名空间
- 跨区域复制
- 计算存储分离
- Web控制台
- WASM插件支持

## 技术栈规划
| 组件         | v0.1       | v1.0              |
|--------------|------------|-------------------|
| **网络框架** | axum       | axum + tonic(gRPC)|
| **元数据**   | 内存HashMap| TiKV/Raft集群     |
| **存储引擎** | 本地FS     | 本地FS+纠删码     |
| **数据校验** | SHA-256    | 多级校验和        |
| **部署**     | 单进程     | K8s Operator      |
