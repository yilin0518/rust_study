# 后端基础、项目实验与阅读入口

更新日期：2026-10-13。任务计划使用下列资料编号；先读任务需要的部分，再实现和验证，不要求整本读完。

## 资料索引

| 编号 | 推荐资料与阅读重点 | 使用方式 |
|---|---|---|
| H | [MDN HTTP 概述](https://developer.mozilla.org/zh-CN/docs/Web/HTTP/Guides/Overview)、[状态码](https://developer.mozilla.org/zh-CN/docs/Web/HTTP/Reference/Status)、[方法](https://developer.mozilla.org/zh-CN/docs/Web/HTTP/Reference/Methods) | 必读：请求/响应、方法、状态码、Header、幂等；HTTP/2 与 TLS 细节选读 |
| A | [Axum 0.7.9](https://docs.rs/axum/0.7.9/axum/) 的 Routing、Handlers、Extractors、Responses、State、Error handling、Middleware | 前两周必读；现有 axum_1 锁文件为 0.7.9，示例必须匹配该版本 |
| T | [Tokio 共享状态](https://tokio.rs/tokio/tutorial/shared-state)、[优雅关闭](https://tokio.rs/tokio/topics/shutdown) | 锁作用域与共享状态先复习；关闭章节第 9 周必读，Runtime 源码选读 |
| S | [SQLx](https://docs.rs/sqlx/latest/sqlx/) 的 MySqlPool、query/query_as、Transaction、migrate；[官方仓库](https://github.com/launchbadge/sqlx) | 第 2 周起必读；添加依赖时核对实际版本、features、MSRV；查询宏所需数据库/离线元数据要单独理解 |
| M | [MySQL 8.4 教程](https://dev.mysql.com/doc/refman/8.4/en/tutorial.html)、[InnoDB 事务](https://dev.mysql.com/doc/refman/8.4/en/innodb-transaction-model.html)、[锁](https://dev.mysql.com/doc/refman/8.4/en/innodb-locking.html)、[优化](https://dev.mysql.com/doc/refman/8.4/en/optimization.html) | 第 2～5 周 SQL、事务、索引必读；按容器实际版本选对应手册，MVCC 内部细节选读 |
| R | [Redis Quick starts](https://redis.io/docs/latest/develop/get-started/)、[数据类型](https://redis.io/docs/latest/develop/data-types/)、[过期命令](https://redis.io/docs/latest/commands/expire/)、[淘汰](https://redis.io/docs/latest/develop/reference/eviction/) | 第 7 周按需阅读；先验证缓存命中/失效/故障，不先学集群 |
| D | [Docker Compose](https://docs.docker.com/compose/gettingstarted/) | 第 2 周本地数据库，第 9 周全服务部署；区分重建容器与删除数据卷 |
| C | [GitHub Actions 构建与测试 Rust](https://docs.github.com/en/actions/tutorials/build-and-test-code/rust) | 第 2 周基础检查，第 9 周数据库集成测试；只写配置，不默认上传或触发外部服务 |
| O | [AWS Transactional Outbox](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/transactional-outbox.html) | 第 8 周必读：双写失败、同事务写事件、重复投递；不要求使用 AWS |
| U | OWASP [认证](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)、[密码存储](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)、[会话管理](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html) | 第 6 周按任务必读：凭据、会话、过期与安全传输；先实现一种会话方案，按 Cookie/Bearer 的选择补相应浏览器边界 |
| Q | [RabbitMQ Tutorials](https://www.rabbitmq.com/tutorials) | 可选：第 8 周后核心任务完成时选一个 Broker；不同时部署 Kafka/RabbitMQ |
| V | [tracing](https://docs.rs/tracing/latest/tracing/)、[Prometheus 指标类型](https://prometheus.io/docs/concepts/metric_types/) | 日志从第一周开始；第 10 周指标类型、计数与延迟分布必读 |
| L | [Linux man-pages](https://man7.org/linux/man-pages/)、[epoll](https://man7.org/linux/man-pages/man7/epoll.7.html)、[proc](https://man7.org/linux/man-pages/man5/proc.5.html) | 按需查进程、socket、内存；epoll 结合已有 TCP 经验复习，内核源码选读 |
| B | [LeetCode 热题 100](https://leetcode.cn/studyplan/top-100-liked/)、[C++ 标准库参考](https://en.cppreference.com/w/cpp/container.html) | 算法题源与容器查阅；计划中的二分、链表等少数题可能是补充题，不要求全部来自热题 100 |
| J | [腾讯国内后台实习示例](https://zhaotalent-tenyu.ihr.tencent-cloud.com/campus/jobs/2056246658733101057)、[腾讯招聘](https://careers.tencent.com/)、[字节校园招聘](https://jobs.bytedance.com/campus/)、[美团校园招聘](https://job.meituan.com/web/campus) | 用于岗位抽样，不代表所有岗位；每次申请核对当前要求与有效性 |

旧 crate 不因为重新规划而统一升级。当前课程沿用 Axum 0.7 的路径参数语法 `/:id`；若学习者后续选择 0.8，应按对应版本迁移为 `/{id}` 并验证行为，不混用示例。

## 知识优先级与证据

| 优先级与知识 | 项目关联 | 实验/解释证据 | 面试问题 |
|---|---|---|---|
| P0 HTTP 方法、JSON、状态码、幂等 | 商品与订单 API | 同路径不同方法、非法输入、重复请求 | PUT 与 POST 的语义有何差别？ |
| P0 TCP、连接复用、超时、DNS、TLS 基本职责 | curl 与服务通信 | 区分连接失败、HTTP 错误、响应丢失 | 响应没收到，服务端一定没执行吗？ |
| P0 进程线程、阻塞、锁、死锁、虚拟内存 | Handler、连接池、任务 | 锁作用域与慢查询影响；画资源等待关系 | 为什么 async 中的阻塞仍影响其他请求？ |
| P0 SQL、约束、索引、事务、隔离、锁、EXPLAIN | 防超卖与订单查询 | 并发竞争、回滚、索引前后执行计划 | 事务为什么不自动消除所有竞态？ |
| P0 Redis、TTL、缓存失效、热点 | 商品查询缓存 | Redis 不可用、更新失效、空值缓存 | 数据库成功但缓存删除失败怎么办？ |
| P0 Linux/Git/构建/部署/测试 | 本地与容器服务 | 查进程端口、日志、环境配置；独立复现 | 如何定位服务无法访问？ |
| P1 状态机、重试、幂等、至少一次投递 | 订单与通知任务 | 超时后重试、重复消息、重启恢复 | 数据库与 Broker 如何避免双写丢事件？ |
| P1 限流、背压、熔断、RPC/负载均衡 | 慢依赖与过载 | 有界队列、连接池饱和、任务积压 | 谁在排队，队列满后怎么处理？ |
| P1 系统设计 | 单体服务演进 | 需求、容量、数据与失败路径 | 扩容后内存限流与缓存有什么变化？ |
| P2 HTTP/2/TLS 深层、零拷贝、epoll 内部、MVCC 源码、Tokio 调度 | 由性能/研究问题驱动 | 具体瓶颈或岗位追问支持学习 | 当前方案的开销究竟来自哪里？ |

不要只背结论。每个知识点保留一张卡：问题、自己的解释、一个代码/SQL/观察证据、限制、下次复习时间。实现功能时同步补对应知识，不为完整目录停下项目数周。

## 独立完成标准

给出没有事先背过的变体问题，能说明假设、方案、边界和验证方法。表达按“为什么需要 → 怎样实现 → 失败时怎样 → 如何验证”组织。读过资料与代码编译通过分别记录，不能直接标记为能够独立解释。
