# Rove 首版需求与设计

状态：2026-09-21 阶段归档，完成 84/93 项，剩余 9 项由后续小提案承接，未标记完成。变更：`bootstrap-rove`。见 [归档说明](archive-summary.md)、[后续计划](../../../../docs/next-iterations.md)、[实现记录](../../../../docs/implementation-status.md) 与 [验收映射](../../../../docs/acceptance-map.json)。

本目录保存历史设计和接口快照；后续行为规格位于 [主规格](../../../specs/)，现行接口位于 [api](../../../../api/README.md)。

| 文档 | 内容 | 建议读者 |
| --- | --- | --- |
| [需求说明](requirements.md) | 产品定位、范围、功能需求、质量要求、验收矩阵 | 产品、开发、测试 |
| [用户故事](user-stories.md) | 用户目标、主流程、异常流程、验收条件 | 产品、界面、测试 |
| [架构设计](design.md) | 技术决策、架构、模块模型、进程模型、数据模型、部署模型与时序 | 开发、部署 |
| [接口说明](api/README.md) | API 分组、调用规则、错误、幂等、流式恢复、契约维护 | SDK、前后端、测试 |
| [Agent OpenAPI](api/rove-agent.openapi.json) | 机器可读 HTTP 接口、共享 schema、socket envelope schema | SDK、agent、GUI/CLI |
| [配置服务 OpenAPI](api/rove-config-server.openapi.json) | 密文上传、下载和到期契约 | 配置服务、agent |
| [本机 socket 映射](api/local-socket.md) | 帧、operationId 映射、远端目标与事件订阅 | SDK、agent |
| [能力规格](specs/) | 七份规范性需求及 WHEN/THEN 场景 | 开发、测试 |
| [任务清单](tasks.md) | 按依赖排序的实现工作 | 开发 |
| [提案](proposal.md) | 变更动机、能力与影响 | 审阅者 |

需求行为以能力规格为准，字段和消息结构以 OpenAPI 为准；需求说明和用户故事负责串联场景，设计说明实现方式。修改接口时同步更新相关规格、故事和任务，不能只更新其中一份。所有状态、默认值和接口均为本提案的设计约定，不表示已经交付。
