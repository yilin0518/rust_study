# 学习主题：HTTP、Axum 与内存商品服务

当前入口：W01-D1。承接已读完的 Rust/Tokio 教程，在已有 `examples/axum_1` 上继续。这个主题可以分多次完成，不按自然日拆成多个文件。

## 1. 本主题要解决什么问题

Echo 服务只把字节写回；商品服务需要让客户端表达“查询列表、创建商品、查询详情”等意图，并返回可理解的结果。HTTP 提供方法、目标、头和响应状态；JSON 是表示数据的一种格式。HTTP 与 JSON 是不同层次：有些 HTTP 响应没有 JSON，有些 JSON 数据也不通过 HTTP 传输。

应用仍由 Tokio runtime 驱动。Axum 为 HTTP 请求匹配路由、提取参数并调用 Handler，让你专注于接口与业务状态。本阶段观察以下链路：

```text
TCP 连接上的 HTTP 请求
  → HTTP 解析
  → Router 匹配路径与方法
  → 提取 Path / Query / State / Json
  → Handler 读取或改变商品状态
  → IntoResponse 转成状态、Header 和 body
  → HTTP 响应
```

不是每条失败请求都会到达 Handler：路径/方法匹配和参数提取本身可以拒绝请求。不要把响应失败都归因于业务代码。

## 2. 前置、版本与范围

- 前置：所有权/借用、Result、Arc/Mutex、async/await、Tokio task。遇到问题针对性回顾，不重读全部教程。
- 本仓库当前 `examples/axum_1/Cargo.toml` 使用 Axum 0.7，锁文件解析为 0.7.9。这个主题按 [对应官方文档](https://docs.rs/axum/0.7.9/axum/) 阅读，路径参数使用 `/:id`。
- 后续若选择 Axum 0.8，需要按版本调整路径语法等 API；这次规划不修改已有依赖。
- 本主题只完成内存商品服务与测试。MySQL 在下一主题，订单在之后。
- 不要求先设计 Repository trait、DDD、微服务或消息队列。接口明确后按实际复杂度提取函数。

### 开始前的依赖判断

先查看当前 manifest 与 Cargo.lock。JSON 需要 Serde 的序列化/反序列化能力，路由测试可能需要 Tower 的测试工具。只有实现相关任务时才添加并解释依赖，选择与现有 Axum 版本相容的版本/features，不照搬最新示例。

## 3. 先预测现有程序

阅读 `examples/axum_1/src/main.rs`，在运行前记录判断：

1. `GET /`、`GET /missing`、`POST /` 是否都会进入 root Handler？各自可能返回什么？
2. 监听地址 `0.0.0.0:3000` 与客户端访问 `127.0.0.1:3000` 有什么关系？
3. 一个 Handler 等待异步操作时，是否意味着整个服务不能处理其他请求？什么情况下仍可能阻塞其他请求？
4. 当前程序只有一个 Router，是否意味着只能处理一个 TCP 连接？

先写理由，不查答案；之后观察框架实际响应，区分预测、现象和解释。

## 4. 增量任务与接口约定

### 增量 A：保留现有入口，增加两条只读接口（W01-D1）

先实现 `GET /healthz`，返回简单存活响应。再实现 `GET /products`，返回一个含 1～2 个固定商品的 JSON 数组。健康接口暂不查询任何数据库。

商品最小字段：`id`、`name`、`price_cents`、`stock`。价格以固定货币的分为整数表示。D1 可以先返回固定列表，D3 再引入共享可变状态。

实现前写明：成功状态码、Content-Type、数组为空时的 body、字段类型。提示入口：`Router::route`、`routing::get`、`Json`、Serde `Serialize`。不要为这一步提前接数据库。

验收：三条只读路径可访问；截图不是唯一证据，优先保存 curl 的响应头与 body，并解释 Router/Handler/JSON 各自负责什么。

### 增量 B：详情查询（W01-D2）

增加 `GET /products/:id`。使用 `Path` 把路由参数提取为数字 ID。找到商品、商品不存在、ID 格式无法解析是不同情况。

先预测：非法数字 ID 会不会进入你的查询函数？它与合法但不存在的 ID 有什么区别？再查 `Path` 文档并实验。

约定：商品存在 200、不存在 404、ID 格式错误 400；框架默认错误 body 是否满足项目约定需要测试。提示入口：`Path`、`StatusCode`、`IntoResponse`、`Result`。

### 增量 C：共享状态与创建（W01-D3）

将商品集合放入 AppState，通过 `State` 提取。实现 `POST /products`，成功 201；验证 name 非空、价格非负、库存有效、字段长度/数值范围有上限。创建输入与查询输出可以是不同类型，客户端不能自行决定由服务端分配的 ID。

先预测：如果每次请求都新建一个 HashMap，上一请求创建的商品还在哪里？如果 clone AppState，哪些数据会被复制，哪些资源会被共享？

所有权模型：Router 持有共享状态，Handler 访问同一集合。Arc 提供共享所有权，锁协调可变访问；Arc 本身不让内部值自动支持并发修改。访问时持有短临界区，得到返回数据后及时释放 guard。

选择锁时从访问模式出发：内存操作短、没有 await 的临界区可以考虑标准库 Mutex；需要跨 await 的等待要重新设计作用域或选择适合的异步机制。不要把所有锁替换成 tokio::Mutex 当成通用修复。先说明 guard 活多久，再选 API。

验收：一个请求创建后，另一个请求能查询；非法输入不改变集合；解释返回数据为什么不能借用已经释放的 guard。

### 增量 D：更新、删除和错误分类（W01-D4）

完成内存版本 `PUT /products/:id`、`DELETE /products/:id`。写清不存在时行为、更新完整字段还是部分字段。订单上线后库存不能随意被更新接口覆盖，下一版本会收紧约定。

设计错误表：输入错误、商品不存在、业务冲突、内部失败。再设计最小 ApiError 转响应方式。无需一开始引入复杂全局错误框架。

先预测：同一个删除请求发两次，业务状态与响应码是否必须完全相同？HTTP 幂等讨论的是预期效果，不应只凭“响应码相同”判断。

### 增量 E：测试与边界（W01-D5～D7）

先列最容易失败的情况再写测试：空列表、详情不存在、非法 ID、缺少字段、非法 JSON、空名称、数值超范围、错误方法、重复 ID、创建后查询、更新后查询、删除后查询。

每个测试获得独立 AppState，避免共享可变全局对象造成顺序依赖。可以测试 Router 而不占用真实端口；网络层行为另用 curl 验证。区分测试对象是业务函数、路由转换还是 TCP 网络，不为一个简单变更复制大量同构测试。

状态码与错误 schema 要覆盖框架拒绝和自己的 ApiError。暂时没有实现统一提取器错误时，在 README 记录缺口，不声称契约全部完成。

## 5. 运行与观察

在仓库根目录运行；如果开发环境编译失败，先保存教育性错误并解释规则，不立即让助手把实现全部替换。

```powershell
cargo check -p axum_1
cargo run -p axum_1
```

服务运行时，在另一个终端逐条执行。Windows 使用 `curl.exe`，避免 PowerShell 某些版本中的 curl 别名影响命令行为。下面是请求命令，不是预先给出的实验结果。

```powershell
curl.exe -i http://127.0.0.1:3000/
curl.exe -i http://127.0.0.1:3000/missing
curl.exe -i -X POST http://127.0.0.1:3000/
curl.exe -i http://127.0.0.1:3000/healthz
curl.exe -i http://127.0.0.1:3000/products
```

写请求体时先保存自己的 JSON 文件，再使用 `--data-binary '@request.json'`；路径相对执行终端，避免复杂引号让 JSON 内容被 shell 改写。测试文件可用虚构数据，不含真实凭据。

记录表：

| 请求 | 事前预测与理由 | 实际状态/响应类型 | Handler 是否执行的证据 | 解释/下一实验 |
|---|---|---|---|---|
| 待填写 | — | — | — | — |

写完测试后执行：

```powershell
cargo test -p axum_1
cargo fmt --check -p axum_1
```

有新增依赖或代码疑问时再运行适合的 clippy/check。这个学习主题不要求助手现在启动服务器或替你观察还未预测的行为。

## 6. 常见错误与提示阶梯

| 现象 | 先检查的模型 | 下一步 |
|---|---|---|
| Handler trait 错误 | 参数提取顺序、返回类型、Future 是否 Send | 缩小到单个 Handler；需要时查版本相符的 debug_handler |
| 请求总是 404 | 路由路径与真实 URL、版本语法 | 先验证固定路径，再加参数 |
| Json 无法编译 | 输出 Serialize、输入 Deserialize | 检查具体字段与 derive，不随意改成字符串 |
| 持 guard 跨 await 引发错误或阻塞 | 借用与临界区、线程迁移 | 画 guard 生命周期，尝试在 await 前结束作用域 |
| 创建后查询为空 | 状态是否共享、是否重建集合 | 找集合创建点和所有权持有者 |
| 测试单独过、一起失败 | 共享状态或固定端口 | 让测试各自拥有隔离状态 |

提示级别：1 概念；2 API 名称；3 数据流/伪代码；4 困难局部片段；5 完整解法仅在明确请求或认真尝试仍卡住后提供。代码 review 顺序为正确性、Rust 语义、系统行为、工程质量、必要性能分析。

## 7. 反思与主题验收

不看代码解释：

1. Router、Handler、extractor 和 IntoResponse 分别承担什么？
2. 数据谁拥有，创建和查询为什么能看到同一份状态？
3. guard 在哪里释放，哪些操作可能让 runtime 工作线程受阻？
4. 重启后内存数据为什么消失，下一主题接 MySQL 要解决什么问题？
5. 哪些失败发生在业务函数之外，测试如何验证？

完成证据：自己的 CRUD 代码、边界测试与命令结果、正确预测或纠正过的解释、README 中的范围与限制。能独立添加一条类似接口后，再标记 V0 验收并进入持久化主题。

## 8. 阅读顺序

- 必读：[HTTP 概述](https://developer.mozilla.org/zh-CN/docs/Web/HTTP/Guides/Overview)，聚焦消息结构与职责；30 分钟预算只读当前任务相关部分。
- 必读：[Axum 0.7.9](https://docs.rs/axum/0.7.9/axum/) 的 Routing、Handlers、Extractors、Responses、Sharing state；边做边查。
- 必读/复习：[Tokio shared state](https://tokio.rs/tokio/tutorial/shared-state)，聚焦 guard 与 await。
- 选读：Axum middleware 与 Tower；在错误映射或日志任务出现时再查。

算法和面试穿插任务见 [W01](../roadmap_12_weeks.md)，结果填写 [进度表](../progress.md)。当前先回答 §3，再实现增量 A，不一次完成本主题全部实现。
