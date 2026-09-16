# prototype-review-handoff Specification

## Purpose

建立从功能组织讨论、浏览器原型评审到现有应用接入的明确交付边界，使确认过的页面与交互可以复用，同时防止模拟验收、规划文件完成和真实平台功能完成被混淆。

## Requirements

### Requirement: Information architecture review before implementation
原型实现前 MUST 记录用户确认的首页、一级导航、详情层级和目标切换规则；候选方案 MUST 标注为待讨论，不以提案生成成功代替设计确认。

#### Scenario: Proposal exists but navigation is undecided
- **WHEN** 规划制品已经生成但用户仍在讨论功能组织
- **THEN** 该方案仍为讨论初稿，不开始页面实现或宣称导航已获批准

### Requirement: Explicit prototype approval before integration
修改现有 GUI 前 MUST 有原型版本、评审结果及用户明确确认记录，并得到后续接入授权；原型验收 MUST 与实际 SDK/平台验收分开。

#### Scenario: User requests prototype adjustments
- **WHEN** 用户查看预览后要求调整布局或交互
- **THEN** 调整仍发生在原型阶段，现有应用保持不变，不自动进入真实后端接入

### Requirement: Reusable and contract-aware handoff
交接 MUST 提供确认的页面/组件清单、主题约定、交互与既有 OpenAPI/CLI 能力对应关系及未验证项；未有协议支持的设计 MUST 明确列出，不能隐藏为模拟成功。

#### Scenario: Prototype is approved
- **WHEN** 用户确认一个原型版本并准备后续接入
- **THEN** 交接材料能够区分可复用 UI、模拟数据、已有接口和仍需讨论的能力，不把浏览器演示计为真实跨设备测试通过
