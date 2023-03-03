## Rust 后端通用包

### 适用于

- `tower` 生态 (`tonic`, `rocket`, `axum` 等)
- `tokio` 异步运行时

### 封装

- 服务基础抽象组件
  - CQRS(Command Query Responsibility Segregation)
  - Resolver per Service
- Http 中间件
  - 身份识别 (Jwt/自定义)
  - Casbin 访问权限管理
- 服务中间件
  - Redis
  - Etcd
  - Consul
  - Rabbitmq
- 服务注册发现
  - etcd (注册/发现)
  - consul (注册)
- 错误处理
  - gRPC Status
- 配置管理
- ...

### TODO

- 支持 cloudwego 生态

### 案例

- https://github.com/iGxnon/douban-rs