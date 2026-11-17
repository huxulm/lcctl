# LeetCode.cn GraphQL API 探索笔记

本文档记录了在 `https://leetcode.cn` 真实浏览器会话及请求验证中观测到的常用只读 GraphQL 接口契约，作为 `lcctl` CLI 工具的接口依据。

## 1. 端点与公共请求头

### GraphQL 端点
- **主网关**：`POST https://leetcode.cn/graphql/`
- **Go 服务网关（`noj-go`）**：`POST https://leetcode.cn/graphql/noj-go/`（用于用户状态、打卡连胜、日历、语言统计、最近 AC、竞赛积分等）
- **通知服务**：`POST https://leetcode.cn/graphql/noty`（如 `followCounts`、`notyActivityConfig`）

### 常用请求头
- `content-type: application/json`（必需）
- `origin: https://leetcode.cn/`
- `referer: https://leetcode.cn/problemset/`（或当前题目页 URL）
- `x-operation-name: <operationName>`
- `random-uuid: <uuid-v4>`
- `cookie`: 包含 `LEETCODE_SESSION=<...>; csrftoken=<...>`（需要登录态的接口必传）
- `x-csrftoken: <csrftoken>`（可选/建议携带）

> 注：只读 GraphQL 查询可通过 `LEETCODE_SESSION` Cookie 完成鉴权；在浏览器只读查询中未强制要求 `x-csrftoken`，但客户端统一透传以兼容更多接口。

---

## 2. 已验证核心接口清单

| Operation Name | 端点 | 是否需登录 | 用途 |
|---|---|---|---|
| `globalData` | `/graphql/noj-go/` | 否（未登录返回匿名状态） | 获取 `userStatus`（`isSignedIn`, `username`, `userSlug`, `realName`, `checkedInToday`, `isPremium`, `activeSessionId`） |
| `getStreakCounter` | `/graphql/noj-go/` | 是（未登录返回 `null`） | 获取当前打卡连胜 `problemsetStreakCounter`（`streakCount`, `todayCompleted`, `daysSkipped`, `today`） |
| `questionOfTodayV2` | `/graphql/` | 否（登录后附带 `userStatus`） | 获取每日一题 `todayRecord`（日期、完成状态、题目基础信息与标签） |
| `dailyQuestionRecords` | `/graphql/` | 否（登录后附带 `userStatus`） | 按年月查询每日一题记录 `dailyQuestionRecords(year, month)` |
| `questionDetail` | `/graphql/` | 否（VIP 题正文需权限） | 按 `titleSlug` 获取题目详情、中英文题面 HTML、样例、`codeSnippets`、提示 |
| `problemsetQuestionListV2` | `/graphql/` | **是**（未登录报错 `请先注册/登录`） | 题库搜索与分页列表；注意 `filters` 需包含 `filterCombineType: "ALL"` |
| `submissionList` | `/graphql/` | 是（未登录返回空列表） | 查询某题提交记录 `submissionList(questionSlug, offset, limit, lastKey)` |
| `submissionDetails` | `/graphql/` | 是（未登录返回 `null`） | 查询单次提交的代码与执行详情 `submissionDetail(submissionId)` |
| `userProfileCalendar` | `/graphql/noj-go/` | 否 | 查询用户刷题日历与连胜 `userCalendar(userSlug, year)` |
| `recentAcSubmissions` | `/graphql/noj-go/` | 否 | 查询最近 AC 记录 `recentACSubmissions(userSlug)` |
| `userProfileUserQuestionProgressV2` | `/graphql/` | 否 | 查询各难度已通过/失败/未做数量及击败比例 |
| `languageStats` | `/graphql/noj-go/` | 否 | 查询各编程语言解题数 `userLanguageProblemCount(userSlug)` |
| `userContestRankingInfo` | `/graphql/noj-go/` | 否 | 查询竞赛积分、参赛次数与排名 `userContestRanking(userSlug)` |

---

## 2.5 已验证积分 REST 接口

- **积分余额**：`GET https://leetcode.cn/points/api/total/`（需登录 Cookie，未登录返回 `{"points": 0}`；登录后返回 `{"points": number}`）。
- **积分明细**：`GET https://leetcode.cn/points/api/`（需登录 Cookie，未登录返回 `401 Unauthorized`；登录后返回 `{"scores": [{ "score": number, "description": string, "date": string }]}`）。

---

## 3. 探索过程中的关键 Schema 差异提醒

1. **`questionOfTodayV2` 与 `QuestionNode` 字段别名**：
   - 实际 GraphQL Schema 字段为 `questionId`、`isPaidOnly`、`translatedName`，前端响应中常用别名为 `id: questionId`、`paidOnly: isPaidOnly`、`nameTranslated: translatedName`。
2. **`problemsetQuestionListV2` 的 `QuestionFilterInput`**：
   - 当传入 `filters` 对象时，必须提供 `filterCombineType`（如 `"ALL"`），传空 `{}` 会触发 `400 Bad Request`。
3. **`submissionList`（`SubmissionDumpNode`）**：
   - 支持 `id`, `title`, `status`, `statusDisplay`, `lang`, `runtime`, `timestamp`, `url`, `isPending`, `memory`, `frontendId`；`langName` 字段在直接查询时会报 Schema 错误。
4. **`submissionDetails`（`SubmissionDetail`）**：
   - `runtime`、`memory` 为直接字段（而非 `runtimeDisplay`/`memoryDisplay`），且支持 `code`, `timestamp`, `statusDisplay`, `isMine`, `lang`, `langVerboseName`, `runtimePercentile`, `memoryPercentile`, `passedTestCaseCnt`, `totalTestCaseCnt`, `stdOutput`, `fullCodeOutput`, `question { questionId titleSlug }`。
5. **难度字段大小写不一致**：
   - `questionOfTodayV2` 返回 `"Easy"` / `"Medium"` / `"Hard"`，而 `problemsetQuestionListV2` 与 `userProfileUserQuestionProgressV2` 返回 `"EASY"` / `"MEDIUM"` / `"HARD"`。
