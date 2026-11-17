use serde::{Deserialize, Deserializer, Serialize};

/// Generic GraphQL envelope returned by `leetcode.cn` endpoints.
#[derive(Debug, Deserialize)]
pub struct GraphQlResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQlError {
    pub message: String,
}

/// Deserializes an ID field that LeetCode may return as either a JSON string or integer.
fn deserialize_flexible_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt {
        Str(String),
        Int(u64),
    }

    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::Str(s) => Ok(s),
        StringOrInt::Int(n) => Ok(n.to_string()),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalDataPayload {
    pub user_status: UserStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserStatus {
    pub is_signed_in: bool,
    pub username: String,
    pub user_slug: Option<String>,
    pub real_name: Option<String>,
    pub checked_in_today: bool,
    pub is_premium: bool,
    pub active_session_id: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakCounterPayload {
    pub problemset_streak_counter: Option<StreakCounter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakCounter {
    pub days_skipped: u32,
    pub streak_count: u32,
    pub today: String,
    pub today_completed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOfTodayPayload {
    pub today_record: Vec<TodayRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayRecord {
    pub date: String,
    pub user_status: Option<String>,
    pub question: DailyQuestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyQuestion {
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub id: String,
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub translated_title: Option<String>,
    pub difficulty: String,
    pub ac_rate: f64,
    pub paid_only: bool,
    pub status: Option<String>,
    #[serde(default)]
    pub topic_tags: Vec<TopicTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicTag {
    pub name: String,
    #[serde(alias = "translatedName")]
    pub name_translated: Option<String>,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRecordsPayload {
    pub daily_question_records: Vec<DailyRecordEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRecordEntry {
    pub date: String,
    pub user_status: Option<String>,
    pub question: DailyRecordQuestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyRecordQuestion {
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub translated_title: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionDetailPayload {
    pub question: Option<QuestionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionDetail {
    pub question_id: String,
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub translated_title: Option<String>,
    pub difficulty: String,
    pub content: Option<String>,
    pub translated_content: Option<String>,
    pub sample_test_case: Option<String>,
    pub example_testcases: Option<String>,
    pub is_paid_only: bool,
    #[serde(default)]
    pub topic_tags: Vec<TopicTag>,
    #[serde(default)]
    pub code_snippets: Vec<CodeSnippet>,
    #[serde(default)]
    pub hints: Vec<String>,
    pub stats: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSnippet {
    pub lang: String,
    pub lang_slug: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemsetListPayload {
    pub problemset_question_list_v2: Option<ProblemsetQuestionListV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemsetQuestionListV2 {
    pub questions: Vec<ProblemListItem>,
    pub total_length: Option<u64>,
    pub has_more: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemListItem {
    #[serde(deserialize_with = "deserialize_flexible_id")]
    pub id: String,
    pub question_frontend_id: String,
    pub title: String,
    pub title_slug: String,
    pub translated_title: Option<String>,
    pub difficulty: String,
    pub paid_only: bool,
    pub ac_rate: f64,
    pub status: Option<String>,
    #[serde(default)]
    pub topic_tags: Vec<TopicTag>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionListPayload {
    pub submission_list: Option<SubmissionList>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionList {
    pub last_key: Option<String>,
    pub has_next: bool,
    pub submissions: Vec<SubmissionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionItem {
    pub id: String,
    pub title: String,
    pub status: String,
    pub status_display: String,
    pub lang: String,
    pub runtime: String,
    pub timestamp: String,
    pub url: String,
    pub is_pending: String,
    pub memory: String,
    pub frontend_id: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionDetailPayload {
    pub submission_detail: Option<SubmissionDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionDetail {
    pub code: String,
    pub timestamp: i64,
    pub status_display: String,
    pub is_mine: bool,
    pub runtime: String,
    pub memory: String,
    pub lang: String,
    pub lang_verbose_name: String,
    pub runtime_percentile: Option<f64>,
    pub memory_percentile: Option<f64>,
    pub passed_test_case_cnt: Option<u32>,
    pub total_test_case_cnt: Option<u32>,
    pub std_output: Option<String>,
    pub full_code_output: Option<String>,
    pub question: SubmissionQuestionRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionQuestionRef {
    pub question_id: String,
    pub title_slug: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCalendarPayload {
    pub user_calendar: Option<UserCalendar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCalendar {
    pub streak: u32,
    pub total_active_days: u32,
    pub submission_calendar: String,
    #[serde(default)]
    pub active_years: Vec<i32>,
    pub recent_streak: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAcPayload {
    #[serde(rename = "recentACSubmissions")]
    pub recent_ac_submissions: Vec<RecentAcSubmission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAcSubmission {
    pub submission_id: u64,
    pub submit_time: i64,
    pub question: RecentAcQuestion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentAcQuestion {
    pub title: String,
    pub translated_title: Option<String>,
    pub title_slug: String,
    pub question_frontend_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProgressPayload {
    pub user_profile_user_question_progress_v2: Option<UserQuestionProgressV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserQuestionProgressV2 {
    pub num_accepted_questions: Vec<DifficultyCount>,
    pub num_failed_questions: Vec<DifficultyCount>,
    pub num_untouched_questions: Vec<DifficultyCount>,
    pub user_session_beats_percentage: Vec<DifficultyBeats>,
    pub total_question_beats_percentage: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyCount {
    pub count: u64,
    pub difficulty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyBeats {
    pub difficulty: String,
    pub percentage: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageStatsPayload {
    pub user_language_problem_count: Vec<LanguageProblemCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageProblemCount {
    pub language_name: String,
    pub problems_solved: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestRankingPayload {
    pub user_contest_ranking: Option<UserContestRanking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserContestRanking {
    pub attended_contests_count: u32,
    pub rating: f64,
    pub global_ranking: u64,
    pub local_ranking: u64,
    pub global_total_participants: u64,
    pub local_total_participants: u64,
    pub top_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsTotal {
    pub points: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointsHistory {
    #[serde(default)]
    pub scores: Vec<PointScoreItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointScoreItem {
    pub score: i64,
    pub description: String,
    pub date: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_today_record_and_numeric_problem_list_id() {
        let daily_json = r#"{
            "todayRecord": [{
                "date": "2026-11-17",
                "userStatus": null,
                "question": {
                    "acRate": 0.569,
                    "difficulty": "Hard",
                    "id": "2879",
                    "paidOnly": false,
                    "questionFrontendId": "2911",
                    "status": null,
                    "title": "Minimum Changes",
                    "titleSlug": "minimum-changes",
                    "translatedTitle": "最少修改次数",
                    "topicTags": [{"name": "String", "nameTranslated": "字符串", "slug": "string"}]
                }
            }]
        }"#;
        let parsed: QuestionOfTodayPayload = serde_json::from_str(daily_json).unwrap();
        assert_eq!(parsed.today_record[0].question.id, "2879");

        let list_json = r#"{
            "problemsetQuestionListV2": {
                "questions": [{
                    "id": 1,
                    "questionFrontendId": "1",
                    "title": "Two Sum",
                    "titleSlug": "two-sum",
                    "translatedTitle": "两数之和",
                    "difficulty": "EASY",
                    "paidOnly": false,
                    "acRate": 0.54,
                    "status": "SOLVED",
                    "topicTags": [{"name": "Array", "nameTranslated": "数组", "slug": "array"}]
                }],
                "totalLength": 1,
                "hasMore": false
            }
        }"#;
        let list: ProblemsetListPayload = serde_json::from_str(list_json).unwrap();
        assert_eq!(list.problemset_question_list_v2.unwrap().questions[0].id, "1");

        let pts_total: PointsTotal = serde_json::from_str(r#"{"points": 10426}"#).unwrap();
        assert_eq!(pts_total.points, 10426);

        let pts_hist: PointsHistory = serde_json::from_str(
            r#"{"scores":[{"score":10,"description":"每日 1 题打卡奖励","date":"2026-03-27T03:02:03.816007Z"}]}"#,
        )
        .unwrap();
        assert_eq!(pts_hist.scores.len(), 1);
        assert_eq!(pts_hist.scores[0].score, 10);
    }
}
