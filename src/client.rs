use crate::config::Credentials;
use crate::models::{
    ContestRankingPayload, DailyRecordEntry, DailyRecordsPayload, GlobalDataPayload,
    GraphQlResponse, LanguageProblemCount, LanguageStatsPayload, PointsHistory, PointsTotal,
    ProblemsetListPayload, ProblemsetQuestionListV2, QuestionDetail, QuestionDetailPayload,
    QuestionOfTodayPayload, RecentAcPayload, RecentAcSubmission, StreakCounter,
    StreakCounterPayload, SubmissionDetail, SubmissionDetailPayload, SubmissionList,
    SubmissionListPayload, TodayRecord, UserCalendar, UserCalendarPayload, UserContestRanking,
    UserProgressPayload, UserQuestionProgressV2, UserStatus,
};
use anyhow::{Context, Result, bail};
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use uuid::Uuid;

const DEFAULT_BASE_URL: &str = "https://leetcode.cn";
const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) lcctl/0.1.0";

#[derive(Debug, Clone, Copy)]
pub enum GraphQlEndpoint {
    Main,
    NojGo,
}

impl GraphQlEndpoint {
    fn path(self) -> &'static str {
        match self {
            Self::Main => "/graphql/",
            Self::NojGo => "/graphql/noj-go/",
        }
    }
}

/// Async HTTP / GraphQL client for `leetcode.cn`.
#[derive(Debug, Clone)]
pub struct LeetCodeClient {
    http: reqwest::Client,
    base_url: String,
    credentials: Credentials,
}

impl LeetCodeClient {
    /// Creates a client targeting `https://leetcode.cn` with the given credentials.
    pub fn new(credentials: Credentials) -> Result<Self> {
        Self::with_base_url(DEFAULT_BASE_URL, credentials)
    }

    /// Creates a client with a custom base URL (useful for testing).
    pub fn with_base_url(base_url: impl Into<String>, credentials: Credentials) -> Result<Self> {
        let http = reqwest::Client::builder()
            .build()
            .context("Failed to initialize HTTP client")?;
        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            credentials,
        })
    }

    pub fn has_session(&self) -> bool {
        self.credentials.has_session()
    }

    /// Executes a GraphQL POST request against `/graphql/` or `/graphql/noj-go/`.
    pub async fn graphql<T: DeserializeOwned>(
        &self,
        endpoint: GraphQlEndpoint,
        operation_name: &str,
        query: &str,
        variables: Value,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, endpoint.path());
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
        headers.insert(
            ORIGIN,
            HeaderValue::from_str(&format!("{}/", self.base_url))?,
        );
        headers.insert(
            REFERER,
            HeaderValue::from_str(&format!("{}/problemset/", self.base_url))?,
        );
        headers.insert("x-operation-name", HeaderValue::from_str(operation_name)?);
        headers.insert(
            "random-uuid",
            HeaderValue::from_str(&Uuid::new_v4().to_string())?,
        );

        if let Some(cookie) = self.credentials.cookie_header() {
            headers.insert(COOKIE, HeaderValue::from_str(&cookie)?);
        }
        if let Some(csrf) = &self.credentials.csrftoken
            && !csrf.trim().is_empty()
        {
            headers.insert("x-csrftoken", HeaderValue::from_str(csrf.trim())?);
        }

        let body = json!({
            "operationName": operation_name,
            "variables": variables,
            "query": query,
        });

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Request to {url} ({operation_name}) failed"))?;

        let status = response.status();
        let raw_text = response
            .text()
            .await
            .with_context(|| format!("Failed reading response body from {url}"))?;

        if !status.is_success() {
            bail!(
                "HTTP {} from {} ({operation_name}): {}",
                status,
                url,
                raw_text.chars().take(400).collect::<String>()
            );
        }

        let envelope: GraphQlResponse<T> = serde_json::from_str(&raw_text).with_context(|| {
            format!(
                "Failed to decode JSON for {operation_name}: {}",
                raw_text.chars().take(400).collect::<String>()
            )
        })?;

        if let Some(errors) = envelope.errors
            && !errors.is_empty()
        {
            let msgs: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
            bail!("GraphQL error in {operation_name}: {}", msgs.join("; "));
        }

        envelope
            .data
            .with_context(|| format!("Missing `data` field in {operation_name} response"))
    }

    /// Executes an authenticated GET JSON request against a REST endpoint on `leetcode.cn`.
    pub async fn get_json<T: DeserializeOwned>(&self, path: &str, referer_path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DEFAULT_USER_AGENT));
        headers.insert(
            REFERER,
            HeaderValue::from_str(&format!("{}{}", self.base_url, referer_path))?,
        );
        headers.insert(
            "x-requested-with",
            HeaderValue::from_static("XMLHttpRequest"),
        );

        if let Some(cookie) = self.credentials.cookie_header() {
            headers.insert(COOKIE, HeaderValue::from_str(&cookie)?);
        }
        if let Some(csrf) = &self.credentials.csrftoken
            && !csrf.trim().is_empty()
        {
            headers.insert("x-csrftoken", HeaderValue::from_str(csrf.trim())?);
        }

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .with_context(|| format!("Request to {url} failed"))?;

        let status = response.status();
        let raw_text = response
            .text()
            .await
            .with_context(|| format!("Failed reading response body from {url}"))?;

        if !status.is_success() {
            bail!(
                "HTTP {} from {}: {}",
                status,
                url,
                raw_text.chars().take(400).collect::<String>()
            );
        }

        serde_json::from_str(&raw_text).with_context(|| {
            format!(
                "Failed to decode JSON from {url}: {}",
                raw_text.chars().take(400).collect::<String>()
            )
        })
    }

    /// Queries the current authenticated user's status via `globalData`.
    pub async fn global_data(&self) -> Result<UserStatus> {
        const QUERY: &str = "query globalData { userStatus { isSignedIn username userSlug realName checkedInToday isPremium activeSessionId } }";
        let payload: GlobalDataPayload = self
            .graphql(GraphQlEndpoint::NojGo, "globalData", QUERY, json!({}))
            .await?;
        Ok(payload.user_status)
    }

    /// Queries the current user's daily check-in streak (`getStreakCounter`).
    pub async fn streak_counter(&self) -> Result<Option<StreakCounter>> {
        const QUERY: &str = "query getStreakCounter { problemsetStreakCounter { daysSkipped streakCount today todayCompleted } }";
        let payload: StreakCounterPayload = self
            .graphql(GraphQlEndpoint::NojGo, "getStreakCounter", QUERY, json!({}))
            .await?;
        Ok(payload.problemset_streak_counter)
    }

    /// Fetches today's daily challenge (`questionOfTodayV2`).
    pub async fn question_of_today(&self) -> Result<Vec<TodayRecord>> {
        const QUERY: &str = r#"
query questionOfTodayV2 {
  todayRecord {
    date
    userStatus
    question {
      acRate
      difficulty
      id: questionId
      paidOnly: isPaidOnly
      questionFrontendId
      status
      title
      titleSlug
      translatedTitle
      topicTags {
        name
        nameTranslated: translatedName
        slug
      }
    }
  }
}"#;
        let payload: QuestionOfTodayPayload = self
            .graphql(GraphQlEndpoint::Main, "questionOfTodayV2", QUERY, json!({}))
            .await?;
        Ok(payload.today_record)
    }

    /// Fetches monthly daily question history (`dailyQuestionRecords`).
    pub async fn daily_question_records(&self, year: i32, month: u32) -> Result<Vec<DailyRecordEntry>> {
        const QUERY: &str = r#"
query dailyQuestionRecords($year: Int!, $month: Int!) {
  dailyQuestionRecords(year: $year, month: $month) {
    date
    userStatus
    question {
      questionFrontendId
      title
      titleSlug
      translatedTitle
    }
  }
}"#;
        let payload: DailyRecordsPayload = self
            .graphql(
                GraphQlEndpoint::Main,
                "dailyQuestionRecords",
                QUERY,
                json!({ "year": year, "month": month }),
            )
            .await?;
        Ok(payload.daily_question_records)
    }

    /// Fetches full problem details and starter code snippets (`questionDetail`).
    pub async fn question_detail(&self, title_slug: &str) -> Result<QuestionDetail> {
        const QUERY: &str = r#"
query questionDetail($titleSlug: String!) {
  question(titleSlug: $titleSlug) {
    questionId
    questionFrontendId
    title
    titleSlug
    translatedTitle
    difficulty
    content
    translatedContent
    sampleTestCase
    exampleTestcases
    isPaidOnly
    topicTags {
      name
      translatedName
      slug
    }
    codeSnippets {
      lang
      langSlug
      code
    }
    hints
    stats
  }
}"#;
        let payload: QuestionDetailPayload = self
            .graphql(
                GraphQlEndpoint::Main,
                "questionDetail",
                QUERY,
                json!({ "titleSlug": title_slug }),
            )
            .await?;
        payload
            .question
            .with_context(|| format!("Problem `{title_slug}` was not found"))
    }

    /// Searches or lists problems using `problemsetQuestionListV2` (requires authentication).
    pub async fn problemset_question_list(
        &self,
        search_keyword: Option<&str>,
        limit: u32,
        skip: u32,
    ) -> Result<ProblemsetQuestionListV2> {
        const QUERY: &str = r#"
query problemsetQuestionListV2($filters: QuestionFilterInput, $limit: Int, $searchKeyword: String, $skip: Int, $sortBy: QuestionSortByInput, $categorySlug: String) {
  problemsetQuestionListV2(
    filters: $filters
    limit: $limit
    searchKeyword: $searchKeyword
    skip: $skip
    sortBy: $sortBy
    categorySlug: $categorySlug
  ) {
    questions {
      id
      titleSlug
      title
      translatedTitle
      questionFrontendId
      paidOnly
      difficulty
      acRate
      status
      topicTags {
        name
        nameTranslated
        slug
      }
    }
    totalLength
    hasMore
  }
}"#;
        let payload: ProblemsetListPayload = self
            .graphql(
                GraphQlEndpoint::Main,
                "problemsetQuestionListV2",
                QUERY,
                json!({
                    "categorySlug": "all-code-essentials",
                    "skip": skip,
                    "limit": limit,
                    "filters": { "filterCombineType": "ALL" },
                    "searchKeyword": search_keyword.unwrap_or(""),
                }),
            )
            .await?;
        payload
            .problemset_question_list_v2
            .context("Empty `problemsetQuestionListV2` response (check login credentials)")
    }

    /// Resolves either a `titleSlug` (e.g. `two-sum`) or a numeric frontend ID (e.g. `1`) to `titleSlug`.
    pub async fn resolve_title_slug(&self, target: &str) -> Result<String> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            bail!("Problem identifier cannot be empty");
        }
        if !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Ok(trimmed.to_lowercase());
        }
        if !self.has_session() {
            bail!(
                "Looking up numeric problem ID `{trimmed}` requires a logged-in session (`lcctl auth login` or `lcctl auth set`). Pass a title slug (e.g. `two-sum`) or configure credentials first."
            );
        }
        let list = self.problemset_question_list(Some(trimmed), 20, 0).await?;
        if let Some(item) = list
            .questions
            .into_iter()
            .find(|q| q.question_frontend_id == trimmed)
        {
            return Ok(item.title_slug);
        }
        bail!("Could not find a problem with frontend ID `{trimmed}`");
    }

    /// Queries the user's submissions for a given problem (`submissionList`).
    pub async fn submission_list(
        &self,
        question_slug: &str,
        offset: u32,
        limit: u32,
        last_key: Option<&str>,
    ) -> Result<SubmissionList> {
        const QUERY: &str = r#"
query submissionList($questionSlug: String!, $offset: Int!, $limit: Int!, $lastKey: String) {
  submissionList(questionSlug: $questionSlug, offset: $offset, limit: $limit, lastKey: $lastKey) {
    lastKey
    hasNext
    submissions {
      id
      title
      status
      statusDisplay
      lang
      runtime
      timestamp
      url
      isPending
      memory
      frontendId
    }
  }
}"#;
        let payload: SubmissionListPayload = self
            .graphql(
                GraphQlEndpoint::Main,
                "submissionList",
                QUERY,
                json!({
                    "questionSlug": question_slug,
                    "offset": offset,
                    "limit": limit,
                    "lastKey": last_key,
                }),
            )
            .await?;
        payload
            .submission_list
            .context("Empty `submissionList` response (authentication may be required)")
    }

    /// Queries full details and source code for a single submission (`submissionDetails`).
    pub async fn submission_detail(&self, submission_id: &str) -> Result<SubmissionDetail> {
        const QUERY: &str = r#"
query submissionDetails($submissionId: ID!) {
  submissionDetail(submissionId: $submissionId) {
    code
    timestamp
    statusDisplay
    isMine
    runtime
    memory
    lang
    langVerboseName
    runtimePercentile
    memoryPercentile
    passedTestCaseCnt
    totalTestCaseCnt
    stdOutput
    fullCodeOutput
    question {
      questionId
      titleSlug
    }
  }
}"#;
        let payload: SubmissionDetailPayload = self
            .graphql(
                GraphQlEndpoint::Main,
                "submissionDetails",
                QUERY,
                json!({ "submissionId": submission_id }),
            )
            .await?;
        payload.submission_detail.with_context(|| {
            format!("Submission `{submission_id}` not found or requires owner authentication")
        })
    }

    /// Queries a user's check-in / submission calendar (`userProfileCalendar`).
    pub async fn user_calendar(
        &self,
        user_slug: &str,
        year: Option<i32>,
    ) -> Result<Option<UserCalendar>> {
        const QUERY: &str = r#"
query userProfileCalendar($userSlug: String!, $year: Int) {
  userCalendar(userSlug: $userSlug, year: $year) {
    streak
    totalActiveDays
    submissionCalendar
    activeYears
    recentStreak
  }
}"#;
        let payload: UserCalendarPayload = self
            .graphql(
                GraphQlEndpoint::NojGo,
                "userProfileCalendar",
                QUERY,
                json!({ "userSlug": user_slug, "year": year }),
            )
            .await?;
        Ok(payload.user_calendar)
    }

    /// Queries a user's recent accepted submissions (`recentAcSubmissions`).
    pub async fn recent_ac_submissions(&self, user_slug: &str) -> Result<Vec<RecentAcSubmission>> {
        const QUERY: &str = r#"
query recentAcSubmissions($userSlug: String!) {
  recentACSubmissions(userSlug: $userSlug) {
    submissionId
    submitTime
    question {
      title
      translatedTitle
      titleSlug
      questionFrontendId
    }
  }
}"#;
        let payload: RecentAcPayload = self
            .graphql(
                GraphQlEndpoint::NojGo,
                "recentAcSubmissions",
                QUERY,
                json!({ "userSlug": user_slug }),
            )
            .await?;
        Ok(payload.recent_ac_submissions)
    }

    /// Queries a user's solved/failed/untouched counts by difficulty.
    pub async fn user_question_progress(
        &self,
        user_slug: &str,
    ) -> Result<Option<UserQuestionProgressV2>> {
        const QUERY: &str = r#"
query userProfileUserQuestionProgressV2($userSlug: String!) {
  userProfileUserQuestionProgressV2(userSlug: $userSlug) {
    numAcceptedQuestions { count difficulty }
    numFailedQuestions { count difficulty }
    numUntouchedQuestions { count difficulty }
    userSessionBeatsPercentage { difficulty percentage }
    totalQuestionBeatsPercentage
  }
}"#;
        let payload: UserProgressPayload = self
            .graphql(
                GraphQlEndpoint::Main,
                "userProfileUserQuestionProgressV2",
                QUERY,
                json!({ "userSlug": user_slug }),
            )
            .await?;
        Ok(payload.user_profile_user_question_progress_v2)
    }

    /// Queries solved problem counts broken down by programming language.
    pub async fn language_stats(&self, user_slug: &str) -> Result<Vec<LanguageProblemCount>> {
        const QUERY: &str = r#"
query languageStats($userSlug: String!) {
  userLanguageProblemCount(userSlug: $userSlug) {
    languageName
    problemsSolved
  }
}"#;
        let payload: LanguageStatsPayload = self
            .graphql(
                GraphQlEndpoint::NojGo,
                "languageStats",
                QUERY,
                json!({ "userSlug": user_slug }),
            )
            .await?;
        Ok(payload.user_language_problem_count)
    }

    /// Queries a user's contest rating and ranking summary.
    pub async fn user_contest_ranking(&self, user_slug: &str) -> Result<Option<UserContestRanking>> {
        const QUERY: &str = r#"
query userContestRankingInfo($userSlug: String!) {
  userContestRanking(userSlug: $userSlug) {
    attendedContestsCount
    rating
    globalRanking
    localRanking
    globalTotalParticipants
    localTotalParticipants
    topPercentage
  }
}"#;
        let payload: ContestRankingPayload = self
            .graphql(
                GraphQlEndpoint::NojGo,
                "userContestRankingInfo",
                QUERY,
                json!({ "userSlug": user_slug }),
            )
            .await?;
        Ok(payload.user_contest_ranking)
    }

    /// Queries the current authenticated user's total points balance (`GET /points/api/total/`).
    pub async fn points_total(&self) -> Result<PointsTotal> {
        self.get_json("/points/api/total/", "/store/").await
    }

    /// Queries the current authenticated user's points earning/spending history (`GET /points/api/`).
    pub async fn points_history(&self) -> Result<PointsHistory> {
        self.get_json("/points/api/", "/settings/points/").await
    }
}
