//! Generated using [cargo-typify](https://github.com/oxidecomputer/typify/tree/main/cargo-typify)
//! from [summary.v3.schema.json](https://github.com/iai-callgrind/iai-callgrind/blob/1c6c01c877cd96a140f7f022e8b61da66a103071/iai-callgrind-runner/schemas/summary.v3.schema.json).
#![allow(dead_code)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::wrong_self_convention)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::match_single_binding)]
#![allow(clippy::clone_on_copy)]

#[doc = r" Error types."]
pub mod error {
    #[doc = r" Error from a `TryFrom` or `FromStr` implementation."]
    pub struct ConversionError(::std::borrow::Cow<'static, str>);
    impl ::std::error::Error for ConversionError {}
    impl ::std::fmt::Display for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Display::fmt(&self.0, f)
        }
    }
    impl ::std::fmt::Debug for ConversionError {
        fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> Result<(), ::std::fmt::Error> {
            ::std::fmt::Debug::fmt(&self.0, f)
        }
    }
    impl From<&'static str> for ConversionError {
        fn from(value: &'static str) -> Self {
            Self(value.into())
        }
    }
    impl From<String> for ConversionError {
        fn from(value: String) -> Self {
            Self(value.into())
        }
    }
}
#[doc = "A `Baseline` depending on the [`BaselineKind`] which points to the corresponding path\n\n This baseline is used for comparisons with the new output of valgrind tools."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A `Baseline` depending on the [`BaselineKind`] which points to the corresponding path\\n\\n This baseline is used for comparisons with the new output of valgrind tools.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"kind\","]
#[doc = "    \"path\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"kind\": {"]
#[doc = "      \"description\": \"The kind of the `Baseline`\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/BaselineKind\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"path\": {"]
#[doc = "      \"description\": \"The path to the file which is used to compare against the new output\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Baseline {
    #[doc = "The kind of the `Baseline`"]
    pub kind: BaselineKind,
    #[doc = "The path to the file which is used to compare against the new output"]
    pub path: ::std::string::String,
}
impl ::std::convert::From<&Baseline> for Baseline {
    fn from(value: &Baseline) -> Self {
        value.clone()
    }
}
impl Baseline {
    pub fn builder() -> builder::Baseline {
        Default::default()
    }
}
#[doc = "The `BaselineKind` describing the baseline"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `BaselineKind` describing the baseline\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"Compare new against `*.old` output files\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Old\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Compare new against a named baseline\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Name\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Name\": {"]
#[doc = "          \"$ref\": \"#/definitions/BaselineName\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum BaselineKind {
    #[doc = "Compare new against `*.old` output files"]
    Old,
    #[doc = "Compare new against a named baseline"]
    Name(BaselineName),
}
impl ::std::convert::From<&Self> for BaselineKind {
    fn from(value: &BaselineKind) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<BaselineName> for BaselineKind {
    fn from(value: BaselineName) -> Self {
        Self::Name(value)
    }
}
#[doc = "`BaselineName`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"type\": \"string\""]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct BaselineName(pub ::std::string::String);
impl ::std::ops::Deref for BaselineName {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<BaselineName> for ::std::string::String {
    fn from(value: BaselineName) -> Self {
        value.0
    }
}
impl ::std::convert::From<&BaselineName> for BaselineName {
    fn from(value: &BaselineName) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::string::String> for BaselineName {
    fn from(value: ::std::string::String) -> Self {
        Self(value)
    }
}
impl ::std::str::FromStr for BaselineName {
    type Err = ::std::convert::Infallible;
    fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
        Ok(Self(value.to_string()))
    }
}
impl ::std::fmt::Display for BaselineName {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        self.0.fmt(f)
    }
}
#[doc = "The `BenchmarkKind`, differentiating between library and binary benchmarks"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `BenchmarkKind`, differentiating between library and binary benchmarks\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"A library benchmark\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LibraryBenchmark\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"A binary benchmark\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"BinaryBenchmark\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BenchmarkKind {
    #[doc = "A library benchmark"]
    LibraryBenchmark,
    #[doc = "A binary benchmark"]
    BinaryBenchmark,
}
impl ::std::convert::From<&Self> for BenchmarkKind {
    fn from(value: &BenchmarkKind) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for BenchmarkKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::LibraryBenchmark => write!(f, "LibraryBenchmark"),
            Self::BinaryBenchmark => write!(f, "BinaryBenchmark"),
        }
    }
}
impl ::std::str::FromStr for BenchmarkKind {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "LibraryBenchmark" => Ok(Self::LibraryBenchmark),
            "BinaryBenchmark" => Ok(Self::BinaryBenchmark),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for BenchmarkKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for BenchmarkKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for BenchmarkKind {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "The `BenchmarkSummary` containing all the information of a single benchmark run\n\n This includes produced files, recorded callgrind events, performance regressions ..."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"BenchmarkSummary\","]
#[doc = "  \"description\": \"The `BenchmarkSummary` containing all the information of a single benchmark run\\n\\n This includes produced files, recorded callgrind events, performance regressions ...\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"benchmark_exe\","]
#[doc = "    \"benchmark_file\","]
#[doc = "    \"function_name\","]
#[doc = "    \"kind\","]
#[doc = "    \"module_path\","]
#[doc = "    \"package_dir\","]
#[doc = "    \"project_root\","]
#[doc = "    \"tool_summaries\","]
#[doc = "    \"version\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"benchmark_exe\": {"]
#[doc = "      \"description\": \"The path to the binary which is executed by valgrind. In case of a library benchmark this\\n is the compiled benchmark file. In case of a binary benchmark this is the path to the\\n command.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"benchmark_file\": {"]
#[doc = "      \"description\": \"The path to the benchmark file\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"callgrind_summary\": {"]
#[doc = "      \"description\": \"The summary of the callgrind run\","]
#[doc = "      \"anyOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/CallgrindSummary\""]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": \"null\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"details\": {"]
#[doc = "      \"description\": \"More details describing this benchmark run\","]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"function_name\": {"]
#[doc = "      \"description\": \"The name of the function under test\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"id\": {"]
#[doc = "      \"description\": \"The user provided id of this benchmark\","]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"kind\": {"]
#[doc = "      \"description\": \"Whether this summary describes a library or binary benchmark\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/BenchmarkKind\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"module_path\": {"]
#[doc = "      \"description\": \"The rust path in the form `bench_file::group::bench`\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"package_dir\": {"]
#[doc = "      \"description\": \"The directory of the package\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"project_root\": {"]
#[doc = "      \"description\": \"The project's root directory\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"summary_output\": {"]
#[doc = "      \"description\": \"The destination and kind of the summary file\","]
#[doc = "      \"anyOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/SummaryOutput\""]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": \"null\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"tool_summaries\": {"]
#[doc = "      \"description\": \"The summary of other valgrind tool runs\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/ToolSummary\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"version\": {"]
#[doc = "      \"description\": \"The version of this format. Only backwards incompatible changes cause an increase of the\\n version\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct BenchmarkSummary {
    #[doc = "The path to the binary which is executed by valgrind. In case of a library benchmark this\n is the compiled benchmark file. In case of a binary benchmark this is the path to the\n command."]
    pub benchmark_exe: ::std::string::String,
    #[doc = "The path to the benchmark file"]
    pub benchmark_file: ::std::string::String,
    #[doc = "The summary of the callgrind run"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub callgrind_summary: ::std::option::Option<CallgrindSummary>,
    #[doc = "More details describing this benchmark run"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub details: ::std::option::Option<::std::string::String>,
    #[doc = "The name of the function under test"]
    pub function_name: ::std::string::String,
    #[doc = "The user provided id of this benchmark"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub id: ::std::option::Option<::std::string::String>,
    #[doc = "Whether this summary describes a library or binary benchmark"]
    pub kind: BenchmarkKind,
    #[doc = "The rust path in the form `bench_file::group::bench`"]
    pub module_path: ::std::string::String,
    #[doc = "The directory of the package"]
    pub package_dir: ::std::string::String,
    #[doc = "The project's root directory"]
    pub project_root: ::std::string::String,
    #[doc = "The destination and kind of the summary file"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub summary_output: ::std::option::Option<SummaryOutput>,
    #[doc = "The summary of other valgrind tool runs"]
    pub tool_summaries: ::std::vec::Vec<ToolSummary>,
    #[doc = "The version of this format. Only backwards incompatible changes cause an increase of the\n version"]
    pub version: ::std::string::String,
}
impl ::std::convert::From<&BenchmarkSummary> for BenchmarkSummary {
    fn from(value: &BenchmarkSummary) -> Self {
        value.clone()
    }
}
impl BenchmarkSummary {
    pub fn builder() -> builder::BenchmarkSummary {
        Default::default()
    }
}
#[doc = "The `CallgrindRegression` describing a single event based performance regression"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `CallgrindRegression` describing a single event based performance regression\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"diff_pct\","]
#[doc = "    \"event_kind\","]
#[doc = "    \"limit\","]
#[doc = "    \"new\","]
#[doc = "    \"old\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"diff_pct\": {"]
#[doc = "      \"description\": \"The difference between new and old in percent. Serialized as string to preserve infinity\\n values and avoid null in json.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"event_kind\": {"]
#[doc = "      \"description\": \"The [`EventKind`] which is affected by a performance regression\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/EventKind\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"limit\": {"]
#[doc = "      \"description\": \"The value of the limit which was exceeded to cause a performance regression. Serialized as\\n string to preserve infinity values and avoid null in json.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"new\": {"]
#[doc = "      \"description\": \"The value of the new benchmark run\","]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"format\": \"uint64\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"old\": {"]
#[doc = "      \"description\": \"The value of the old benchmark run\","]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"format\": \"uint64\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct CallgrindRegression {
    #[doc = "The difference between new and old in percent. Serialized as string to preserve infinity\n values and avoid null in json."]
    pub diff_pct: ::std::string::String,
    #[doc = "The [`EventKind`] which is affected by a performance regression"]
    pub event_kind: EventKind,
    #[doc = "The value of the limit which was exceeded to cause a performance regression. Serialized as\n string to preserve infinity values and avoid null in json."]
    pub limit: ::std::string::String,
    #[doc = "The value of the new benchmark run"]
    pub new: u64,
    #[doc = "The value of the old benchmark run"]
    pub old: u64,
}
impl ::std::convert::From<&CallgrindRegression> for CallgrindRegression {
    fn from(value: &CallgrindRegression) -> Self {
        value.clone()
    }
}
impl CallgrindRegression {
    pub fn builder() -> builder::CallgrindRegression {
        Default::default()
    }
}
#[doc = "The `CallgrindRun` contains all `CallgrindRunSegments` and their total costs in a\n `CallgrindTotal`."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `CallgrindRun` contains all `CallgrindRunSegments` and their total costs in a\\n `CallgrindTotal`.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"segments\","]
#[doc = "    \"total\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"segments\": {"]
#[doc = "      \"description\": \"All `CallgrindRunSummary`s\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/CallgrindRunSegment\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"total\": {"]
#[doc = "      \"description\": \"The total costs of all `CallgrindRunSummary`s in this `CallgrindRunSummaries`\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/CallgrindTotal\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct CallgrindRun {
    #[doc = "All `CallgrindRunSummary`s"]
    pub segments: ::std::vec::Vec<CallgrindRunSegment>,
    #[doc = "The total costs of all `CallgrindRunSummary`s in this `CallgrindRunSummaries`"]
    pub total: CallgrindTotal,
}
impl ::std::convert::From<&CallgrindRun> for CallgrindRun {
    fn from(value: &CallgrindRun) -> Self {
        value.clone()
    }
}
impl CallgrindRun {
    pub fn builder() -> builder::CallgrindRun {
        Default::default()
    }
}
#[doc = "The `CallgrindRunSegment` containing the metric differences, performance regressions of a\n callgrind run segment.\n\n A segment can be a part (caused by options like `--dump-every-bb=xxx`), a thread (caused by\n `--separate-threads`) or a pid (possibly caused by `--trace-children`). A segment is a summary\n over a single file which contains the costs of that part, thread and/or pid."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `CallgrindRunSegment` containing the metric differences, performance regressions of a\\n callgrind run segment.\\n\\n A segment can be a part (caused by options like `--dump-every-bb=xxx`), a thread (caused by\\n `--separate-threads`) or a pid (possibly caused by `--trace-children`). A segment is a summary\\n over a single file which contains the costs of that part, thread and/or pid.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"command\","]
#[doc = "    \"events\","]
#[doc = "    \"regressions\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"baseline\": {"]
#[doc = "      \"description\": \"If present, the `Baseline` used to compare the new with the old output\","]
#[doc = "      \"anyOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/Baseline\""]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": \"null\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"command\": {"]
#[doc = "      \"description\": \"The executed command extracted from Valgrind output\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"events\": {"]
#[doc = "      \"description\": \"All recorded metrics for the `EventKinds`\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary_for_EventKind\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"regressions\": {"]
#[doc = "      \"description\": \"All detected performance regressions per callgrind run\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/CallgrindRegression\""]
#[doc = "      }"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct CallgrindRunSegment {
    #[doc = "If present, the `Baseline` used to compare the new with the old output"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub baseline: ::std::option::Option<Baseline>,
    #[doc = "The executed command extracted from Valgrind output"]
    pub command: ::std::string::String,
    #[doc = "All recorded metrics for the `EventKinds`"]
    pub events: MetricsSummaryForEventKind,
    #[doc = "All detected performance regressions per callgrind run"]
    pub regressions: ::std::vec::Vec<CallgrindRegression>,
}
impl ::std::convert::From<&CallgrindRunSegment> for CallgrindRunSegment {
    fn from(value: &CallgrindRunSegment) -> Self {
        value.clone()
    }
}
impl CallgrindRunSegment {
    pub fn builder() -> builder::CallgrindRunSegment {
        Default::default()
    }
}
#[doc = "The `CallgrindSummary` contains the callgrind run, flamegraph paths and other paths to the\n segments of the callgrind run."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `CallgrindSummary` contains the callgrind run, flamegraph paths and other paths to the\\n segments of the callgrind run.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"callgrind_run\","]
#[doc = "    \"flamegraphs\","]
#[doc = "    \"log_paths\","]
#[doc = "    \"out_paths\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"callgrind_run\": {"]
#[doc = "      \"description\": \"The summary of all callgrind segments is a `CallgrindRun`\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/CallgrindRun\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"flamegraphs\": {"]
#[doc = "      \"description\": \"The summaries of possibly created flamegraphs\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/FlamegraphSummary\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"log_paths\": {"]
#[doc = "      \"description\": \"The paths to the `*.log` files\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"out_paths\": {"]
#[doc = "      \"description\": \"The paths to the `*.out` files\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct CallgrindSummary {
    #[doc = "The summary of all callgrind segments is a `CallgrindRun`"]
    pub callgrind_run: CallgrindRun,
    #[doc = "The summaries of possibly created flamegraphs"]
    pub flamegraphs: ::std::vec::Vec<FlamegraphSummary>,
    #[doc = "The paths to the `*.log` files"]
    pub log_paths: ::std::vec::Vec<::std::string::String>,
    #[doc = "The paths to the `*.out` files"]
    pub out_paths: ::std::vec::Vec<::std::string::String>,
}
impl ::std::convert::From<&CallgrindSummary> for CallgrindSummary {
    fn from(value: &CallgrindSummary) -> Self {
        value.clone()
    }
}
impl CallgrindSummary {
    pub fn builder() -> builder::CallgrindSummary {
        Default::default()
    }
}
#[doc = "The total callgrind costs over the `CallgrindRunSegments` and all detected regressions for the\n total"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The total callgrind costs over the `CallgrindRunSegments` and all detected regressions for the\\n total\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"regressions\","]
#[doc = "    \"summary\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"regressions\": {"]
#[doc = "      \"description\": \"All detected regressions for the total metrics\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/CallgrindRegression\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"summary\": {"]
#[doc = "      \"description\": \"The total over the segment metrics\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary_for_EventKind\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct CallgrindTotal {
    #[doc = "All detected regressions for the total metrics"]
    pub regressions: ::std::vec::Vec<CallgrindRegression>,
    #[doc = "The total over the segment metrics"]
    pub summary: MetricsSummaryForEventKind,
}
impl ::std::convert::From<&CallgrindTotal> for CallgrindTotal {
    fn from(value: &CallgrindTotal) -> Self {
        value.clone()
    }
}
impl CallgrindTotal {
    pub fn builder() -> builder::CallgrindTotal {
        Default::default()
    }
}
#[doc = "The differences between two `Metrics` as percentage and factor"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The differences between two `Metrics` as percentage and factor\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"diff_pct\","]
#[doc = "    \"factor\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"diff_pct\": {"]
#[doc = "      \"description\": \"The percentage of the difference between two `Metrics` serialized as string to preserve\\n infinity values and avoid `null` in json\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"factor\": {"]
#[doc = "      \"description\": \"The factor of the difference between two `Metrics` serialized as string to preserve\\n infinity values and void `null` in json\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Diffs {
    #[doc = "The percentage of the difference between two `Metrics` serialized as string to preserve\n infinity values and avoid `null` in json"]
    pub diff_pct: ::std::string::String,
    #[doc = "The factor of the difference between two `Metrics` serialized as string to preserve\n infinity values and void `null` in json"]
    pub factor: ::std::string::String,
}
impl ::std::convert::From<&Diffs> for Diffs {
    fn from(value: &Diffs) -> Self {
        value.clone()
    }
}
impl Diffs {
    pub fn builder() -> builder::Diffs {
        Default::default()
    }
}
#[doc = "Either left or right or both can be present\n\n Most of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\n left is `new` and right is `old`."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Either left or right or both can be present\\n\\n Most of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\\n left is `new` and right is `old`.\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"The left or `new` value\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Left\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Left\": {"]
#[doc = "          \"$ref\": \"#/definitions/SegmentDetails\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The right or `old` value\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Right\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Right\": {"]
#[doc = "          \"$ref\": \"#/definitions/SegmentDetails\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Both values (`new` and `old`) are present\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Both\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Both\": {"]
#[doc = "          \"type\": \"array\","]
#[doc = "          \"items\": ["]
#[doc = "            {"]
#[doc = "              \"$ref\": \"#/definitions/SegmentDetails\""]
#[doc = "            },"]
#[doc = "            {"]
#[doc = "              \"$ref\": \"#/definitions/SegmentDetails\""]
#[doc = "            }"]
#[doc = "          ],"]
#[doc = "          \"maxItems\": 2,"]
#[doc = "          \"minItems\": 2"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum EitherOrBothForSegmentDetails {
    #[doc = "The left or `new` value"]
    Left(SegmentDetails),
    #[doc = "The right or `old` value"]
    Right(SegmentDetails),
    #[doc = "Both values (`new` and `old`) are present"]
    Both(SegmentDetails, SegmentDetails),
}
impl ::std::convert::From<&Self> for EitherOrBothForSegmentDetails {
    fn from(value: &EitherOrBothForSegmentDetails) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<(SegmentDetails, SegmentDetails)> for EitherOrBothForSegmentDetails {
    fn from(value: (SegmentDetails, SegmentDetails)) -> Self {
        Self::Both(value.0, value.1)
    }
}
#[doc = "Either left or right or both can be present\n\n Most of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\n left is `new` and right is `old`."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Either left or right or both can be present\\n\\n Most of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\\n left is `new` and right is `old`.\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"The left or `new` value\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Left\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Left\": {"]
#[doc = "          \"type\": \"integer\","]
#[doc = "          \"format\": \"uint64\","]
#[doc = "          \"minimum\": 0.0"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The right or `old` value\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Right\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Right\": {"]
#[doc = "          \"type\": \"integer\","]
#[doc = "          \"format\": \"uint64\","]
#[doc = "          \"minimum\": 0.0"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Both values (`new` and `old`) are present\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Both\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Both\": {"]
#[doc = "          \"type\": \"array\","]
#[doc = "          \"items\": ["]
#[doc = "            {"]
#[doc = "              \"type\": \"integer\","]
#[doc = "              \"format\": \"uint64\","]
#[doc = "              \"minimum\": 0.0"]
#[doc = "            },"]
#[doc = "            {"]
#[doc = "              \"type\": \"integer\","]
#[doc = "              \"format\": \"uint64\","]
#[doc = "              \"minimum\": 0.0"]
#[doc = "            }"]
#[doc = "          ],"]
#[doc = "          \"maxItems\": 2,"]
#[doc = "          \"minItems\": 2"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum EitherOrBothForUint64 {
    #[doc = "The left or `new` value"]
    Left(u64),
    #[doc = "The right or `old` value"]
    Right(u64),
    #[doc = "Both values (`new` and `old`) are present"]
    Both(u64, u64),
}
impl ::std::convert::From<&Self> for EitherOrBothForUint64 {
    fn from(value: &EitherOrBothForUint64) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<(u64, u64)> for EitherOrBothForUint64 {
    fn from(value: (u64, u64)) -> Self {
        Self::Both(value.0, value.1)
    }
}
#[doc = "All `EventKind`s callgrind produces and additionally some derived events\n\n Depending on the options passed to Callgrind, these are the events that Callgrind can produce.\n See the [Callgrind\n documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"All `EventKind`s callgrind produces and additionally some derived events\\n\\n Depending on the options passed to Callgrind, these are the events that Callgrind can produce.\\n See the [Callgrind\\n documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details.\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"The default event. I cache reads (which equals the number of instructions executed)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Ir\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"D Cache reads (which equals the number of memory reads) (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Dr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"D Cache writes (which equals the number of memory writes) (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Dw\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"I1 cache read misses (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"I1mr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"D1 cache read misses (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"D1mr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"D1 cache write misses (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"D1mw\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL cache instruction read misses (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"ILmr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL cache data read misses (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"DLmr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL cache data write misses (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"DLmw\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Derived event showing the L1 hits (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"L1hits\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Derived event showing the LL hits (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLhits\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Derived event showing the RAM hits (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"RamHits\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Derived event showing the total amount of cache reads and writes (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"TotalRW\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Derived event showing estimated CPU cycles (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"EstimatedCycles\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The number of system calls done (--collect-systime=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SysCount\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The elapsed time spent in system calls (--collect-systime=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SysTime\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The cpu time spent during system calls (--collect-systime=nsec)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SysCpuTime\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The number of global bus events (--collect-bus=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Ge\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Conditional branches executed (--branch-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Bc\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Conditional branches mispredicted (--branch-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Bcm\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Indirect branches executed (--branch-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Bi\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Indirect branches mispredicted (--branch-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Bim\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Dirty miss because of instruction read (--simulate-wb=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"ILdmr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Dirty miss because of data read (--simulate-wb=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"DLdmr\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Dirty miss because of data write (--simulate-wb=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"DLdmw\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Counter showing bad temporal locality for L1 caches (--cachuse=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"AcCost1\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Counter showing bad temporal locality for LL caches (--cachuse=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"AcCost2\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Counter showing bad spatial locality for L1 caches (--cachuse=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SpLoss1\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Counter showing bad spatial locality for LL caches (--cachuse=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SpLoss2\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventKind {
    #[doc = "The default event. I cache reads (which equals the number of instructions executed)"]
    Ir,
    #[doc = "D Cache reads (which equals the number of memory reads) (--cache-sim=yes)"]
    Dr,
    #[doc = "D Cache writes (which equals the number of memory writes) (--cache-sim=yes)"]
    Dw,
    #[doc = "I1 cache read misses (--cache-sim=yes)"]
    I1mr,
    #[doc = "D1 cache read misses (--cache-sim=yes)"]
    D1mr,
    #[doc = "D1 cache write misses (--cache-sim=yes)"]
    D1mw,
    #[doc = "LL cache instruction read misses (--cache-sim=yes)"]
    ILmr,
    #[doc = "LL cache data read misses (--cache-sim=yes)"]
    DLmr,
    #[doc = "LL cache data write misses (--cache-sim=yes)"]
    DLmw,
    #[doc = "Derived event showing the L1 hits (--cache-sim=yes)"]
    L1hits,
    #[doc = "Derived event showing the LL hits (--cache-sim=yes)"]
    LLhits,
    #[doc = "Derived event showing the RAM hits (--cache-sim=yes)"]
    RamHits,
    #[doc = "Derived event showing the total amount of cache reads and writes (--cache-sim=yes)"]
    #[serde(rename = "TotalRW")]
    TotalRw,
    #[doc = "Derived event showing estimated CPU cycles (--cache-sim=yes)"]
    EstimatedCycles,
    #[doc = "The number of system calls done (--collect-systime=yes)"]
    SysCount,
    #[doc = "The elapsed time spent in system calls (--collect-systime=yes)"]
    SysTime,
    #[doc = "The cpu time spent during system calls (--collect-systime=nsec)"]
    SysCpuTime,
    #[doc = "The number of global bus events (--collect-bus=yes)"]
    Ge,
    #[doc = "Conditional branches executed (--branch-sim=yes)"]
    Bc,
    #[doc = "Conditional branches mispredicted (--branch-sim=yes)"]
    Bcm,
    #[doc = "Indirect branches executed (--branch-sim=yes)"]
    Bi,
    #[doc = "Indirect branches mispredicted (--branch-sim=yes)"]
    Bim,
    #[doc = "Dirty miss because of instruction read (--simulate-wb=yes)"]
    ILdmr,
    #[doc = "Dirty miss because of data read (--simulate-wb=yes)"]
    DLdmr,
    #[doc = "Dirty miss because of data write (--simulate-wb=yes)"]
    DLdmw,
    #[doc = "Counter showing bad temporal locality for L1 caches (--cachuse=yes)"]
    AcCost1,
    #[doc = "Counter showing bad temporal locality for LL caches (--cachuse=yes)"]
    AcCost2,
    #[doc = "Counter showing bad spatial locality for L1 caches (--cachuse=yes)"]
    SpLoss1,
    #[doc = "Counter showing bad spatial locality for LL caches (--cachuse=yes)"]
    SpLoss2,
}
impl ::std::convert::From<&Self> for EventKind {
    fn from(value: &EventKind) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for EventKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Ir => write!(f, "Ir"),
            Self::Dr => write!(f, "Dr"),
            Self::Dw => write!(f, "Dw"),
            Self::I1mr => write!(f, "I1mr"),
            Self::D1mr => write!(f, "D1mr"),
            Self::D1mw => write!(f, "D1mw"),
            Self::ILmr => write!(f, "ILmr"),
            Self::DLmr => write!(f, "DLmr"),
            Self::DLmw => write!(f, "DLmw"),
            Self::L1hits => write!(f, "L1hits"),
            Self::LLhits => write!(f, "LLhits"),
            Self::RamHits => write!(f, "RamHits"),
            Self::TotalRw => write!(f, "TotalRW"),
            Self::EstimatedCycles => write!(f, "EstimatedCycles"),
            Self::SysCount => write!(f, "SysCount"),
            Self::SysTime => write!(f, "SysTime"),
            Self::SysCpuTime => write!(f, "SysCpuTime"),
            Self::Ge => write!(f, "Ge"),
            Self::Bc => write!(f, "Bc"),
            Self::Bcm => write!(f, "Bcm"),
            Self::Bi => write!(f, "Bi"),
            Self::Bim => write!(f, "Bim"),
            Self::ILdmr => write!(f, "ILdmr"),
            Self::DLdmr => write!(f, "DLdmr"),
            Self::DLdmw => write!(f, "DLdmw"),
            Self::AcCost1 => write!(f, "AcCost1"),
            Self::AcCost2 => write!(f, "AcCost2"),
            Self::SpLoss1 => write!(f, "SpLoss1"),
            Self::SpLoss2 => write!(f, "SpLoss2"),
        }
    }
}
impl ::std::str::FromStr for EventKind {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "Ir" => Ok(Self::Ir),
            "Dr" => Ok(Self::Dr),
            "Dw" => Ok(Self::Dw),
            "I1mr" => Ok(Self::I1mr),
            "D1mr" => Ok(Self::D1mr),
            "D1mw" => Ok(Self::D1mw),
            "ILmr" => Ok(Self::ILmr),
            "DLmr" => Ok(Self::DLmr),
            "DLmw" => Ok(Self::DLmw),
            "L1hits" => Ok(Self::L1hits),
            "LLhits" => Ok(Self::LLhits),
            "RamHits" => Ok(Self::RamHits),
            "TotalRW" => Ok(Self::TotalRw),
            "EstimatedCycles" => Ok(Self::EstimatedCycles),
            "SysCount" => Ok(Self::SysCount),
            "SysTime" => Ok(Self::SysTime),
            "SysCpuTime" => Ok(Self::SysCpuTime),
            "Ge" => Ok(Self::Ge),
            "Bc" => Ok(Self::Bc),
            "Bcm" => Ok(Self::Bcm),
            "Bi" => Ok(Self::Bi),
            "Bim" => Ok(Self::Bim),
            "ILdmr" => Ok(Self::ILdmr),
            "DLdmr" => Ok(Self::DLdmr),
            "DLdmw" => Ok(Self::DLdmw),
            "AcCost1" => Ok(Self::AcCost1),
            "AcCost2" => Ok(Self::AcCost2),
            "SpLoss1" => Ok(Self::SpLoss1),
            "SpLoss2" => Ok(Self::SpLoss2),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventKind {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventKind {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "The callgrind `FlamegraphSummary` records all created paths for an [`EventKind`] specific\n flamegraph\n\n Either the `regular_path`, `old_path` or the `diff_path` are present. Never can all of them be\n absent."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The callgrind `FlamegraphSummary` records all created paths for an [`EventKind`] specific\\n flamegraph\\n\\n Either the `regular_path`, `old_path` or the `diff_path` are present. Never can all of them be\\n absent.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"event_kind\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"base_path\": {"]
#[doc = "      \"description\": \"If present, the path to the file of the old regular (non-differential) flamegraph\","]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"diff_path\": {"]
#[doc = "      \"description\": \"If present, the path to the file of the differential flamegraph\","]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"event_kind\": {"]
#[doc = "      \"description\": \"The `EventKind` of the flamegraph\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/EventKind\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"regular_path\": {"]
#[doc = "      \"description\": \"If present, the path to the file of the regular (non-differential) flamegraph\","]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct FlamegraphSummary {
    #[doc = "If present, the path to the file of the old regular (non-differential) flamegraph"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub base_path: ::std::option::Option<::std::string::String>,
    #[doc = "If present, the path to the file of the differential flamegraph"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub diff_path: ::std::option::Option<::std::string::String>,
    #[doc = "The `EventKind` of the flamegraph"]
    pub event_kind: EventKind,
    #[doc = "If present, the path to the file of the regular (non-differential) flamegraph"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub regular_path: ::std::option::Option<::std::string::String>,
}
impl ::std::convert::From<&FlamegraphSummary> for FlamegraphSummary {
    fn from(value: &FlamegraphSummary) -> Self {
        value.clone()
    }
}
impl FlamegraphSummary {
    pub fn builder() -> builder::FlamegraphSummary {
        Default::default()
    }
}
#[doc = "The `MetricsDiff` describes the difference between a `new` and `old` metric as percentage and\n factor.\n\n Only if both metrics are present there is also a `Diffs` present. Otherwise, it just stores the\n `new` or `old` metric."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `MetricsDiff` describes the difference between a `new` and `old` metric as percentage and\\n factor.\\n\\n Only if both metrics are present there is also a `Diffs` present. Otherwise, it just stores the\\n `new` or `old` metric.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"metrics\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"diffs\": {"]
#[doc = "      \"description\": \"If both metrics are present there is also a `Diffs` present\","]
#[doc = "      \"anyOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/Diffs\""]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": \"null\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"metrics\": {"]
#[doc = "      \"description\": \"Either the `new`, `old` or both metrics\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/EitherOrBoth_for_uint64\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct MetricsDiff {
    #[doc = "If both metrics are present there is also a `Diffs` present"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub diffs: ::std::option::Option<Diffs>,
    #[doc = "Either the `new`, `old` or both metrics"]
    pub metrics: EitherOrBothForUint64,
}
impl ::std::convert::From<&MetricsDiff> for MetricsDiff {
    fn from(value: &MetricsDiff) -> Self {
        value.clone()
    }
}
impl MetricsDiff {
    pub fn builder() -> builder::MetricsDiff {
        Default::default()
    }
}
#[doc = "The `MetricsSummary` contains all differences between two tool run segments"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `MetricsSummary` contains all differences between two tool run segments\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"additionalProperties\": {"]
#[doc = "    \"$ref\": \"#/definitions/MetricsDiff\""]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct MetricsSummaryForDhatMetricKind(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummaryForDhatMetricKind {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummaryForDhatMetricKind>
    for ::std::collections::HashMap<::std::string::String, MetricsDiff>
{
    fn from(value: MetricsSummaryForDhatMetricKind) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummaryForDhatMetricKind> for MetricsSummaryForDhatMetricKind {
    fn from(value: &MetricsSummaryForDhatMetricKind) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>>
    for MetricsSummaryForDhatMetricKind
{
    fn from(value: ::std::collections::HashMap<::std::string::String, MetricsDiff>) -> Self {
        Self(value)
    }
}
#[doc = "The `MetricsSummary` contains all differences between two tool run segments"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `MetricsSummary` contains all differences between two tool run segments\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"additionalProperties\": {"]
#[doc = "    \"$ref\": \"#/definitions/MetricsDiff\""]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct MetricsSummaryForErrorMetricKind(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummaryForErrorMetricKind {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummaryForErrorMetricKind>
    for ::std::collections::HashMap<::std::string::String, MetricsDiff>
{
    fn from(value: MetricsSummaryForErrorMetricKind) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummaryForErrorMetricKind> for MetricsSummaryForErrorMetricKind {
    fn from(value: &MetricsSummaryForErrorMetricKind) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>>
    for MetricsSummaryForErrorMetricKind
{
    fn from(value: ::std::collections::HashMap<::std::string::String, MetricsDiff>) -> Self {
        Self(value)
    }
}
#[doc = "The `MetricsSummary` contains all differences between two tool run segments"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `MetricsSummary` contains all differences between two tool run segments\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"additionalProperties\": {"]
#[doc = "    \"$ref\": \"#/definitions/MetricsDiff\""]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct MetricsSummaryForEventKind(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummaryForEventKind {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummaryForEventKind> for ::std::collections::HashMap<::std::string::String, MetricsDiff> {
    fn from(value: MetricsSummaryForEventKind) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummaryForEventKind> for MetricsSummaryForEventKind {
    fn from(value: &MetricsSummaryForEventKind) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>> for MetricsSummaryForEventKind {
    fn from(value: ::std::collections::HashMap<::std::string::String, MetricsDiff>) -> Self {
        Self(value)
    }
}
#[doc = "Some additional and necessary information about the tool run segment"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Some additional and necessary information about the tool run segment\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"command\","]
#[doc = "    \"path\","]
#[doc = "    \"pid\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"command\": {"]
#[doc = "      \"description\": \"The executed command extracted from Valgrind output\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"details\": {"]
#[doc = "      \"description\": \"More details for example from the logging output of the tool run\","]
#[doc = "      \"type\": ["]
#[doc = "        \"string\","]
#[doc = "        \"null\""]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"parent_pid\": {"]
#[doc = "      \"description\": \"The parent pid of this process\","]
#[doc = "      \"type\": ["]
#[doc = "        \"integer\","]
#[doc = "        \"null\""]
#[doc = "      ],"]
#[doc = "      \"format\": \"int32\""]
#[doc = "    },"]
#[doc = "    \"part\": {"]
#[doc = "      \"description\": \"The part of this tool run (only callgrind)\","]
#[doc = "      \"type\": ["]
#[doc = "        \"integer\","]
#[doc = "        \"null\""]
#[doc = "      ],"]
#[doc = "      \"format\": \"uint64\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    },"]
#[doc = "    \"path\": {"]
#[doc = "      \"description\": \"The path to the file from the tool run\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"pid\": {"]
#[doc = "      \"description\": \"The pid of this process\","]
#[doc = "      \"type\": \"integer\","]
#[doc = "      \"format\": \"int32\""]
#[doc = "    },"]
#[doc = "    \"thread\": {"]
#[doc = "      \"description\": \"The thread of this tool run (only callgrind)\","]
#[doc = "      \"type\": ["]
#[doc = "        \"integer\","]
#[doc = "        \"null\""]
#[doc = "      ],"]
#[doc = "      \"format\": \"uint\","]
#[doc = "      \"minimum\": 0.0"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct SegmentDetails {
    #[doc = "The executed command extracted from Valgrind output"]
    pub command: ::std::string::String,
    #[doc = "More details for example from the logging output of the tool run"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub details: ::std::option::Option<::std::string::String>,
    #[doc = "The parent pid of this process"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub parent_pid: ::std::option::Option<i32>,
    #[doc = "The part of this tool run (only callgrind)"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub part: ::std::option::Option<u64>,
    #[doc = "The path to the file from the tool run"]
    pub path: ::std::string::String,
    #[doc = "The pid of this process"]
    pub pid: i32,
    #[doc = "The thread of this tool run (only callgrind)"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub thread: ::std::option::Option<u32>,
}
impl ::std::convert::From<&SegmentDetails> for SegmentDetails {
    fn from(value: &SegmentDetails) -> Self {
        value.clone()
    }
}
impl SegmentDetails {
    pub fn builder() -> builder::SegmentDetails {
        Default::default()
    }
}
#[doc = "The format (json, ...) in which the summary file should be saved or printed"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The format (json, ...) in which the summary file should be saved or printed\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"The format in a space optimal json representation without newlines\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Json\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The format in pretty printed json\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"PrettyJson\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SummaryFormat {
    #[doc = "The format in a space optimal json representation without newlines"]
    Json,
    #[doc = "The format in pretty printed json"]
    PrettyJson,
}
impl ::std::convert::From<&Self> for SummaryFormat {
    fn from(value: &SummaryFormat) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for SummaryFormat {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Json => write!(f, "Json"),
            Self::PrettyJson => write!(f, "PrettyJson"),
        }
    }
}
impl ::std::str::FromStr for SummaryFormat {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "Json" => Ok(Self::Json),
            "PrettyJson" => Ok(Self::PrettyJson),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for SummaryFormat {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for SummaryFormat {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for SummaryFormat {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "Manage the summary output file with this `SummaryOutput`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Manage the summary output file with this `SummaryOutput`\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"format\","]
#[doc = "    \"path\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"format\": {"]
#[doc = "      \"description\": \"The [`SummaryFormat`]\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/SummaryFormat\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"path\": {"]
#[doc = "      \"description\": \"The path to the destination file of this summary\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct SummaryOutput {
    #[doc = "The [`SummaryFormat`]"]
    pub format: SummaryFormat,
    #[doc = "The path to the destination file of this summary"]
    pub path: ::std::string::String,
}
impl ::std::convert::From<&SummaryOutput> for SummaryOutput {
    fn from(value: &SummaryOutput) -> Self {
        value.clone()
    }
}
impl SummaryOutput {
    pub fn builder() -> builder::SummaryOutput {
        Default::default()
    }
}
#[doc = "The `ToolMetricSummary` contains the `MetricsSummary` distinguished by tool and metric kinds"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `ToolMetricSummary` contains the `MetricsSummary` distinguished by tool and metric kinds\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"If there are no metrics extracted (currently massif, bbv)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"None\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The error summary of tools which reports errors (memcheck, helgrind, drd)\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"ErrorSummary\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"ErrorSummary\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary_for_ErrorMetricKind\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The dhat summary\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"DhatSummary\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"DhatSummary\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary_for_DhatMetricKind\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The callgrind summary\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"CallgrindSummary\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"CallgrindSummary\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary_for_EventKind\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum ToolMetricSummary {
    #[doc = "If there are no metrics extracted (currently massif, bbv)"]
    None,
    #[doc = "The error summary of tools which reports errors (memcheck, helgrind, drd)"]
    ErrorSummary(MetricsSummaryForErrorMetricKind),
    #[doc = "The dhat summary"]
    DhatSummary(MetricsSummaryForDhatMetricKind),
    #[doc = "The callgrind summary"]
    CallgrindSummary(MetricsSummaryForEventKind),
}
impl ::std::convert::From<&Self> for ToolMetricSummary {
    fn from(value: &ToolMetricSummary) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<MetricsSummaryForErrorMetricKind> for ToolMetricSummary {
    fn from(value: MetricsSummaryForErrorMetricKind) -> Self {
        Self::ErrorSummary(value)
    }
}
impl ::std::convert::From<MetricsSummaryForDhatMetricKind> for ToolMetricSummary {
    fn from(value: MetricsSummaryForDhatMetricKind) -> Self {
        Self::DhatSummary(value)
    }
}
impl ::std::convert::From<MetricsSummaryForEventKind> for ToolMetricSummary {
    fn from(value: MetricsSummaryForEventKind) -> Self {
        Self::CallgrindSummary(value)
    }
}
#[doc = "The `ToolRun` contains all information about a single tool run with possibly multiple segments\n\n The total is always present and summarizes all tool run segments. In the special case of a\n single tool run segment, the total equals the metrics of this segment."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `ToolRun` contains all information about a single tool run with possibly multiple segments\\n\\n The total is always present and summarizes all tool run segments. In the special case of a\\n single tool run segment, the total equals the metrics of this segment.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"segments\","]
#[doc = "    \"total\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"segments\": {"]
#[doc = "      \"description\": \"All `ToolRunSegment`s\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/ToolRunSegment\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"total\": {"]
#[doc = "      \"description\": \"The total over the `ToolRunSegment`s\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/ToolMetricSummary\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ToolRun {
    #[doc = "All `ToolRunSegment`s"]
    pub segments: ::std::vec::Vec<ToolRunSegment>,
    #[doc = "The total over the `ToolRunSegment`s"]
    pub total: ToolMetricSummary,
}
impl ::std::convert::From<&ToolRun> for ToolRun {
    fn from(value: &ToolRun) -> Self {
        value.clone()
    }
}
impl ToolRun {
    pub fn builder() -> builder::ToolRun {
        Default::default()
    }
}
#[doc = "A single segment of a tool run and if present the comparison with the \"old\" segment\n\n A tool run can produce multiple segments, for example for each process and subprocess with\n (--trace-children)."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A single segment of a tool run and if present the comparison with the \\\"old\\\" segment\\n\\n A tool run can produce multiple segments, for example for each process and subprocess with\\n (--trace-children).\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"details\","]
#[doc = "    \"metrics_summary\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"details\": {"]
#[doc = "      \"description\": \"The details (like command, thread number etc.) about the segment(s)\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/EitherOrBoth_for_SegmentDetails\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"metrics_summary\": {"]
#[doc = "      \"description\": \"The `ToolMetricSummary`\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/ToolMetricSummary\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ToolRunSegment {
    #[doc = "The details (like command, thread number etc.) about the segment(s)"]
    pub details: EitherOrBothForSegmentDetails,
    #[doc = "The `ToolMetricSummary`"]
    pub metrics_summary: ToolMetricSummary,
}
impl ::std::convert::From<&ToolRunSegment> for ToolRunSegment {
    fn from(value: &ToolRunSegment) -> Self {
        value.clone()
    }
}
impl ToolRunSegment {
    pub fn builder() -> builder::ToolRunSegment {
        Default::default()
    }
}
#[doc = "The `ToolSummary` containing all information about a valgrind tool run"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `ToolSummary` containing all information about a valgrind tool run\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"log_paths\","]
#[doc = "    \"out_paths\","]
#[doc = "    \"summaries\","]
#[doc = "    \"tool\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"log_paths\": {"]
#[doc = "      \"description\": \"The paths to the `*.log` files. All tools produce at least one log file\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"out_paths\": {"]
#[doc = "      \"description\": \"The paths to the `*.out` files. Not all tools produce an output in addition to the log\\n files\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"summaries\": {"]
#[doc = "      \"description\": \"The metrics and details about the tool run\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/ToolRun\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"tool\": {"]
#[doc = "      \"description\": \"The Valgrind tool like `DHAT`, `Memcheck` etc.\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/ValgrindTool\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ToolSummary {
    #[doc = "The paths to the `*.log` files. All tools produce at least one log file"]
    pub log_paths: ::std::vec::Vec<::std::string::String>,
    #[doc = "The paths to the `*.out` files. Not all tools produce an output in addition to the log\n files"]
    pub out_paths: ::std::vec::Vec<::std::string::String>,
    #[doc = "The metrics and details about the tool run"]
    pub summaries: ToolRun,
    #[doc = "The Valgrind tool like `DHAT`, `Memcheck` etc."]
    pub tool: ValgrindTool,
}
impl ::std::convert::From<&ToolSummary> for ToolSummary {
    fn from(value: &ToolSummary) -> Self {
        value.clone()
    }
}
impl ToolSummary {
    pub fn builder() -> builder::ToolSummary {
        Default::default()
    }
}
#[doc = "All currently available valgrind tools"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"All currently available valgrind tools\","]
#[doc = "  \"type\": \"string\","]
#[doc = "  \"enum\": ["]
#[doc = "    \"Callgrind\","]
#[doc = "    \"Memcheck\","]
#[doc = "    \"Helgrind\","]
#[doc = "    \"DRD\","]
#[doc = "    \"Massif\","]
#[doc = "    \"DHAT\","]
#[doc = "    \"BBV\""]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValgrindTool {
    Callgrind,
    Memcheck,
    Helgrind,
    #[serde(rename = "DRD")]
    Drd,
    Massif,
    #[serde(rename = "DHAT")]
    Dhat,
    #[serde(rename = "BBV")]
    Bbv,
}
impl ::std::convert::From<&Self> for ValgrindTool {
    fn from(value: &ValgrindTool) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for ValgrindTool {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Callgrind => write!(f, "Callgrind"),
            Self::Memcheck => write!(f, "Memcheck"),
            Self::Helgrind => write!(f, "Helgrind"),
            Self::Drd => write!(f, "DRD"),
            Self::Massif => write!(f, "Massif"),
            Self::Dhat => write!(f, "DHAT"),
            Self::Bbv => write!(f, "BBV"),
        }
    }
}
impl ::std::str::FromStr for ValgrindTool {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "Callgrind" => Ok(Self::Callgrind),
            "Memcheck" => Ok(Self::Memcheck),
            "Helgrind" => Ok(Self::Helgrind),
            "DRD" => Ok(Self::Drd),
            "Massif" => Ok(Self::Massif),
            "DHAT" => Ok(Self::Dhat),
            "BBV" => Ok(Self::Bbv),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ValgrindTool {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ValgrindTool {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ValgrindTool {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = r" Types for composing complex structures."]
pub mod builder {
    #[derive(Clone, Debug)]
    pub struct Baseline {
        kind: ::std::result::Result<super::BaselineKind, ::std::string::String>,
        path: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for Baseline {
        fn default() -> Self {
            Self {
                kind: Err("no value supplied for kind".to_string()),
                path: Err("no value supplied for path".to_string()),
            }
        }
    }
    impl Baseline {
        pub fn kind<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::BaselineKind>,
            T::Error: ::std::fmt::Display,
        {
            self.kind = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for kind: {}", e));
            self
        }
        pub fn path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for path: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<Baseline> for super::Baseline {
        type Error = super::error::ConversionError;
        fn try_from(value: Baseline) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                kind: value.kind?,
                path: value.path?,
            })
        }
    }
    impl ::std::convert::From<super::Baseline> for Baseline {
        fn from(value: super::Baseline) -> Self {
            Self {
                kind: Ok(value.kind),
                path: Ok(value.path),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct BenchmarkSummary {
        benchmark_exe: ::std::result::Result<::std::string::String, ::std::string::String>,
        benchmark_file: ::std::result::Result<::std::string::String, ::std::string::String>,
        callgrind_summary: ::std::result::Result<::std::option::Option<super::CallgrindSummary>, ::std::string::String>,
        details: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        function_name: ::std::result::Result<::std::string::String, ::std::string::String>,
        id: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        kind: ::std::result::Result<super::BenchmarkKind, ::std::string::String>,
        module_path: ::std::result::Result<::std::string::String, ::std::string::String>,
        package_dir: ::std::result::Result<::std::string::String, ::std::string::String>,
        project_root: ::std::result::Result<::std::string::String, ::std::string::String>,
        summary_output: ::std::result::Result<::std::option::Option<super::SummaryOutput>, ::std::string::String>,
        tool_summaries: ::std::result::Result<::std::vec::Vec<super::ToolSummary>, ::std::string::String>,
        version: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for BenchmarkSummary {
        fn default() -> Self {
            Self {
                benchmark_exe: Err("no value supplied for benchmark_exe".to_string()),
                benchmark_file: Err("no value supplied for benchmark_file".to_string()),
                callgrind_summary: Ok(Default::default()),
                details: Ok(Default::default()),
                function_name: Err("no value supplied for function_name".to_string()),
                id: Ok(Default::default()),
                kind: Err("no value supplied for kind".to_string()),
                module_path: Err("no value supplied for module_path".to_string()),
                package_dir: Err("no value supplied for package_dir".to_string()),
                project_root: Err("no value supplied for project_root".to_string()),
                summary_output: Ok(Default::default()),
                tool_summaries: Err("no value supplied for tool_summaries".to_string()),
                version: Err("no value supplied for version".to_string()),
            }
        }
    }
    impl BenchmarkSummary {
        pub fn benchmark_exe<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.benchmark_exe = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for benchmark_exe: {}", e));
            self
        }
        pub fn benchmark_file<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.benchmark_file = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for benchmark_file: {}", e));
            self
        }
        pub fn callgrind_summary<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<super::CallgrindSummary>>,
            T::Error: ::std::fmt::Display,
        {
            self.callgrind_summary = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for callgrind_summary: {}", e));
            self
        }
        pub fn details<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.details = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for details: {}", e));
            self
        }
        pub fn function_name<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.function_name = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for function_name: {}", e));
            self
        }
        pub fn id<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.id = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for id: {}", e));
            self
        }
        pub fn kind<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::BenchmarkKind>,
            T::Error: ::std::fmt::Display,
        {
            self.kind = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for kind: {}", e));
            self
        }
        pub fn module_path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.module_path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for module_path: {}", e));
            self
        }
        pub fn package_dir<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.package_dir = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for package_dir: {}", e));
            self
        }
        pub fn project_root<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.project_root = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for project_root: {}", e));
            self
        }
        pub fn summary_output<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<super::SummaryOutput>>,
            T::Error: ::std::fmt::Display,
        {
            self.summary_output = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for summary_output: {}", e));
            self
        }
        pub fn tool_summaries<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::ToolSummary>>,
            T::Error: ::std::fmt::Display,
        {
            self.tool_summaries = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for tool_summaries: {}", e));
            self
        }
        pub fn version<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.version = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for version: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<BenchmarkSummary> for super::BenchmarkSummary {
        type Error = super::error::ConversionError;
        fn try_from(value: BenchmarkSummary) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                benchmark_exe: value.benchmark_exe?,
                benchmark_file: value.benchmark_file?,
                callgrind_summary: value.callgrind_summary?,
                details: value.details?,
                function_name: value.function_name?,
                id: value.id?,
                kind: value.kind?,
                module_path: value.module_path?,
                package_dir: value.package_dir?,
                project_root: value.project_root?,
                summary_output: value.summary_output?,
                tool_summaries: value.tool_summaries?,
                version: value.version?,
            })
        }
    }
    impl ::std::convert::From<super::BenchmarkSummary> for BenchmarkSummary {
        fn from(value: super::BenchmarkSummary) -> Self {
            Self {
                benchmark_exe: Ok(value.benchmark_exe),
                benchmark_file: Ok(value.benchmark_file),
                callgrind_summary: Ok(value.callgrind_summary),
                details: Ok(value.details),
                function_name: Ok(value.function_name),
                id: Ok(value.id),
                kind: Ok(value.kind),
                module_path: Ok(value.module_path),
                package_dir: Ok(value.package_dir),
                project_root: Ok(value.project_root),
                summary_output: Ok(value.summary_output),
                tool_summaries: Ok(value.tool_summaries),
                version: Ok(value.version),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct CallgrindRegression {
        diff_pct: ::std::result::Result<::std::string::String, ::std::string::String>,
        event_kind: ::std::result::Result<super::EventKind, ::std::string::String>,
        limit: ::std::result::Result<::std::string::String, ::std::string::String>,
        new: ::std::result::Result<u64, ::std::string::String>,
        old: ::std::result::Result<u64, ::std::string::String>,
    }
    impl ::std::default::Default for CallgrindRegression {
        fn default() -> Self {
            Self {
                diff_pct: Err("no value supplied for diff_pct".to_string()),
                event_kind: Err("no value supplied for event_kind".to_string()),
                limit: Err("no value supplied for limit".to_string()),
                new: Err("no value supplied for new".to_string()),
                old: Err("no value supplied for old".to_string()),
            }
        }
    }
    impl CallgrindRegression {
        pub fn diff_pct<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.diff_pct = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for diff_pct: {}", e));
            self
        }
        pub fn event_kind<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::EventKind>,
            T::Error: ::std::fmt::Display,
        {
            self.event_kind = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for event_kind: {}", e));
            self
        }
        pub fn limit<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.limit = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for limit: {}", e));
            self
        }
        pub fn new<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u64>,
            T::Error: ::std::fmt::Display,
        {
            self.new = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for new: {}", e));
            self
        }
        pub fn old<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<u64>,
            T::Error: ::std::fmt::Display,
        {
            self.old = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for old: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<CallgrindRegression> for super::CallgrindRegression {
        type Error = super::error::ConversionError;
        fn try_from(value: CallgrindRegression) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                diff_pct: value.diff_pct?,
                event_kind: value.event_kind?,
                limit: value.limit?,
                new: value.new?,
                old: value.old?,
            })
        }
    }
    impl ::std::convert::From<super::CallgrindRegression> for CallgrindRegression {
        fn from(value: super::CallgrindRegression) -> Self {
            Self {
                diff_pct: Ok(value.diff_pct),
                event_kind: Ok(value.event_kind),
                limit: Ok(value.limit),
                new: Ok(value.new),
                old: Ok(value.old),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct CallgrindRun {
        segments: ::std::result::Result<::std::vec::Vec<super::CallgrindRunSegment>, ::std::string::String>,
        total: ::std::result::Result<super::CallgrindTotal, ::std::string::String>,
    }
    impl ::std::default::Default for CallgrindRun {
        fn default() -> Self {
            Self {
                segments: Err("no value supplied for segments".to_string()),
                total: Err("no value supplied for total".to_string()),
            }
        }
    }
    impl CallgrindRun {
        pub fn segments<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::CallgrindRunSegment>>,
            T::Error: ::std::fmt::Display,
        {
            self.segments = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for segments: {}", e));
            self
        }
        pub fn total<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::CallgrindTotal>,
            T::Error: ::std::fmt::Display,
        {
            self.total = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for total: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<CallgrindRun> for super::CallgrindRun {
        type Error = super::error::ConversionError;
        fn try_from(value: CallgrindRun) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                segments: value.segments?,
                total: value.total?,
            })
        }
    }
    impl ::std::convert::From<super::CallgrindRun> for CallgrindRun {
        fn from(value: super::CallgrindRun) -> Self {
            Self {
                segments: Ok(value.segments),
                total: Ok(value.total),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct CallgrindRunSegment {
        baseline: ::std::result::Result<::std::option::Option<super::Baseline>, ::std::string::String>,
        command: ::std::result::Result<::std::string::String, ::std::string::String>,
        events: ::std::result::Result<super::MetricsSummaryForEventKind, ::std::string::String>,
        regressions: ::std::result::Result<::std::vec::Vec<super::CallgrindRegression>, ::std::string::String>,
    }
    impl ::std::default::Default for CallgrindRunSegment {
        fn default() -> Self {
            Self {
                baseline: Ok(Default::default()),
                command: Err("no value supplied for command".to_string()),
                events: Err("no value supplied for events".to_string()),
                regressions: Err("no value supplied for regressions".to_string()),
            }
        }
    }
    impl CallgrindRunSegment {
        pub fn baseline<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<super::Baseline>>,
            T::Error: ::std::fmt::Display,
        {
            self.baseline = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for baseline: {}", e));
            self
        }
        pub fn command<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.command = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for command: {}", e));
            self
        }
        pub fn events<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::MetricsSummaryForEventKind>,
            T::Error: ::std::fmt::Display,
        {
            self.events = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for events: {}", e));
            self
        }
        pub fn regressions<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::CallgrindRegression>>,
            T::Error: ::std::fmt::Display,
        {
            self.regressions = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for regressions: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<CallgrindRunSegment> for super::CallgrindRunSegment {
        type Error = super::error::ConversionError;
        fn try_from(value: CallgrindRunSegment) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                baseline: value.baseline?,
                command: value.command?,
                events: value.events?,
                regressions: value.regressions?,
            })
        }
    }
    impl ::std::convert::From<super::CallgrindRunSegment> for CallgrindRunSegment {
        fn from(value: super::CallgrindRunSegment) -> Self {
            Self {
                baseline: Ok(value.baseline),
                command: Ok(value.command),
                events: Ok(value.events),
                regressions: Ok(value.regressions),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct CallgrindSummary {
        callgrind_run: ::std::result::Result<super::CallgrindRun, ::std::string::String>,
        flamegraphs: ::std::result::Result<::std::vec::Vec<super::FlamegraphSummary>, ::std::string::String>,
        log_paths: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        out_paths: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
    }
    impl ::std::default::Default for CallgrindSummary {
        fn default() -> Self {
            Self {
                callgrind_run: Err("no value supplied for callgrind_run".to_string()),
                flamegraphs: Err("no value supplied for flamegraphs".to_string()),
                log_paths: Err("no value supplied for log_paths".to_string()),
                out_paths: Err("no value supplied for out_paths".to_string()),
            }
        }
    }
    impl CallgrindSummary {
        pub fn callgrind_run<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::CallgrindRun>,
            T::Error: ::std::fmt::Display,
        {
            self.callgrind_run = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for callgrind_run: {}", e));
            self
        }
        pub fn flamegraphs<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::FlamegraphSummary>>,
            T::Error: ::std::fmt::Display,
        {
            self.flamegraphs = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for flamegraphs: {}", e));
            self
        }
        pub fn log_paths<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.log_paths = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for log_paths: {}", e));
            self
        }
        pub fn out_paths<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.out_paths = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for out_paths: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<CallgrindSummary> for super::CallgrindSummary {
        type Error = super::error::ConversionError;
        fn try_from(value: CallgrindSummary) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                callgrind_run: value.callgrind_run?,
                flamegraphs: value.flamegraphs?,
                log_paths: value.log_paths?,
                out_paths: value.out_paths?,
            })
        }
    }
    impl ::std::convert::From<super::CallgrindSummary> for CallgrindSummary {
        fn from(value: super::CallgrindSummary) -> Self {
            Self {
                callgrind_run: Ok(value.callgrind_run),
                flamegraphs: Ok(value.flamegraphs),
                log_paths: Ok(value.log_paths),
                out_paths: Ok(value.out_paths),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct CallgrindTotal {
        regressions: ::std::result::Result<::std::vec::Vec<super::CallgrindRegression>, ::std::string::String>,
        summary: ::std::result::Result<super::MetricsSummaryForEventKind, ::std::string::String>,
    }
    impl ::std::default::Default for CallgrindTotal {
        fn default() -> Self {
            Self {
                regressions: Err("no value supplied for regressions".to_string()),
                summary: Err("no value supplied for summary".to_string()),
            }
        }
    }
    impl CallgrindTotal {
        pub fn regressions<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::CallgrindRegression>>,
            T::Error: ::std::fmt::Display,
        {
            self.regressions = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for regressions: {}", e));
            self
        }
        pub fn summary<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::MetricsSummaryForEventKind>,
            T::Error: ::std::fmt::Display,
        {
            self.summary = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for summary: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<CallgrindTotal> for super::CallgrindTotal {
        type Error = super::error::ConversionError;
        fn try_from(value: CallgrindTotal) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                regressions: value.regressions?,
                summary: value.summary?,
            })
        }
    }
    impl ::std::convert::From<super::CallgrindTotal> for CallgrindTotal {
        fn from(value: super::CallgrindTotal) -> Self {
            Self {
                regressions: Ok(value.regressions),
                summary: Ok(value.summary),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct Diffs {
        diff_pct: ::std::result::Result<::std::string::String, ::std::string::String>,
        factor: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for Diffs {
        fn default() -> Self {
            Self {
                diff_pct: Err("no value supplied for diff_pct".to_string()),
                factor: Err("no value supplied for factor".to_string()),
            }
        }
    }
    impl Diffs {
        pub fn diff_pct<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.diff_pct = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for diff_pct: {}", e));
            self
        }
        pub fn factor<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.factor = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for factor: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<Diffs> for super::Diffs {
        type Error = super::error::ConversionError;
        fn try_from(value: Diffs) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                diff_pct: value.diff_pct?,
                factor: value.factor?,
            })
        }
    }
    impl ::std::convert::From<super::Diffs> for Diffs {
        fn from(value: super::Diffs) -> Self {
            Self {
                diff_pct: Ok(value.diff_pct),
                factor: Ok(value.factor),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct FlamegraphSummary {
        base_path: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        diff_path: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        event_kind: ::std::result::Result<super::EventKind, ::std::string::String>,
        regular_path: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
    }
    impl ::std::default::Default for FlamegraphSummary {
        fn default() -> Self {
            Self {
                base_path: Ok(Default::default()),
                diff_path: Ok(Default::default()),
                event_kind: Err("no value supplied for event_kind".to_string()),
                regular_path: Ok(Default::default()),
            }
        }
    }
    impl FlamegraphSummary {
        pub fn base_path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.base_path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for base_path: {}", e));
            self
        }
        pub fn diff_path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.diff_path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for diff_path: {}", e));
            self
        }
        pub fn event_kind<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::EventKind>,
            T::Error: ::std::fmt::Display,
        {
            self.event_kind = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for event_kind: {}", e));
            self
        }
        pub fn regular_path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.regular_path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for regular_path: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<FlamegraphSummary> for super::FlamegraphSummary {
        type Error = super::error::ConversionError;
        fn try_from(value: FlamegraphSummary) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                base_path: value.base_path?,
                diff_path: value.diff_path?,
                event_kind: value.event_kind?,
                regular_path: value.regular_path?,
            })
        }
    }
    impl ::std::convert::From<super::FlamegraphSummary> for FlamegraphSummary {
        fn from(value: super::FlamegraphSummary) -> Self {
            Self {
                base_path: Ok(value.base_path),
                diff_path: Ok(value.diff_path),
                event_kind: Ok(value.event_kind),
                regular_path: Ok(value.regular_path),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct MetricsDiff {
        diffs: ::std::result::Result<::std::option::Option<super::Diffs>, ::std::string::String>,
        metrics: ::std::result::Result<super::EitherOrBothForUint64, ::std::string::String>,
    }
    impl ::std::default::Default for MetricsDiff {
        fn default() -> Self {
            Self {
                diffs: Ok(Default::default()),
                metrics: Err("no value supplied for metrics".to_string()),
            }
        }
    }
    impl MetricsDiff {
        pub fn diffs<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<super::Diffs>>,
            T::Error: ::std::fmt::Display,
        {
            self.diffs = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for diffs: {}", e));
            self
        }
        pub fn metrics<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::EitherOrBothForUint64>,
            T::Error: ::std::fmt::Display,
        {
            self.metrics = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for metrics: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<MetricsDiff> for super::MetricsDiff {
        type Error = super::error::ConversionError;
        fn try_from(value: MetricsDiff) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                diffs: value.diffs?,
                metrics: value.metrics?,
            })
        }
    }
    impl ::std::convert::From<super::MetricsDiff> for MetricsDiff {
        fn from(value: super::MetricsDiff) -> Self {
            Self {
                diffs: Ok(value.diffs),
                metrics: Ok(value.metrics),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SegmentDetails {
        command: ::std::result::Result<::std::string::String, ::std::string::String>,
        details: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        parent_pid: ::std::result::Result<::std::option::Option<i32>, ::std::string::String>,
        part: ::std::result::Result<::std::option::Option<u64>, ::std::string::String>,
        path: ::std::result::Result<::std::string::String, ::std::string::String>,
        pid: ::std::result::Result<i32, ::std::string::String>,
        thread: ::std::result::Result<::std::option::Option<u32>, ::std::string::String>,
    }
    impl ::std::default::Default for SegmentDetails {
        fn default() -> Self {
            Self {
                command: Err("no value supplied for command".to_string()),
                details: Ok(Default::default()),
                parent_pid: Ok(Default::default()),
                part: Ok(Default::default()),
                path: Err("no value supplied for path".to_string()),
                pid: Err("no value supplied for pid".to_string()),
                thread: Ok(Default::default()),
            }
        }
    }
    impl SegmentDetails {
        pub fn command<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.command = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for command: {}", e));
            self
        }
        pub fn details<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.details = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for details: {}", e));
            self
        }
        pub fn parent_pid<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<i32>>,
            T::Error: ::std::fmt::Display,
        {
            self.parent_pid = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for parent_pid: {}", e));
            self
        }
        pub fn part<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<u64>>,
            T::Error: ::std::fmt::Display,
        {
            self.part = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for part: {}", e));
            self
        }
        pub fn path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for path: {}", e));
            self
        }
        pub fn pid<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<i32>,
            T::Error: ::std::fmt::Display,
        {
            self.pid = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for pid: {}", e));
            self
        }
        pub fn thread<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::option::Option<u32>>,
            T::Error: ::std::fmt::Display,
        {
            self.thread = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for thread: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<SegmentDetails> for super::SegmentDetails {
        type Error = super::error::ConversionError;
        fn try_from(value: SegmentDetails) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                command: value.command?,
                details: value.details?,
                parent_pid: value.parent_pid?,
                part: value.part?,
                path: value.path?,
                pid: value.pid?,
                thread: value.thread?,
            })
        }
    }
    impl ::std::convert::From<super::SegmentDetails> for SegmentDetails {
        fn from(value: super::SegmentDetails) -> Self {
            Self {
                command: Ok(value.command),
                details: Ok(value.details),
                parent_pid: Ok(value.parent_pid),
                part: Ok(value.part),
                path: Ok(value.path),
                pid: Ok(value.pid),
                thread: Ok(value.thread),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct SummaryOutput {
        format: ::std::result::Result<super::SummaryFormat, ::std::string::String>,
        path: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for SummaryOutput {
        fn default() -> Self {
            Self {
                format: Err("no value supplied for format".to_string()),
                path: Err("no value supplied for path".to_string()),
            }
        }
    }
    impl SummaryOutput {
        pub fn format<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::SummaryFormat>,
            T::Error: ::std::fmt::Display,
        {
            self.format = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for format: {}", e));
            self
        }
        pub fn path<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::string::String>,
            T::Error: ::std::fmt::Display,
        {
            self.path = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for path: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<SummaryOutput> for super::SummaryOutput {
        type Error = super::error::ConversionError;
        fn try_from(value: SummaryOutput) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                format: value.format?,
                path: value.path?,
            })
        }
    }
    impl ::std::convert::From<super::SummaryOutput> for SummaryOutput {
        fn from(value: super::SummaryOutput) -> Self {
            Self {
                format: Ok(value.format),
                path: Ok(value.path),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ToolRun {
        segments: ::std::result::Result<::std::vec::Vec<super::ToolRunSegment>, ::std::string::String>,
        total: ::std::result::Result<super::ToolMetricSummary, ::std::string::String>,
    }
    impl ::std::default::Default for ToolRun {
        fn default() -> Self {
            Self {
                segments: Err("no value supplied for segments".to_string()),
                total: Err("no value supplied for total".to_string()),
            }
        }
    }
    impl ToolRun {
        pub fn segments<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::ToolRunSegment>>,
            T::Error: ::std::fmt::Display,
        {
            self.segments = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for segments: {}", e));
            self
        }
        pub fn total<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ToolMetricSummary>,
            T::Error: ::std::fmt::Display,
        {
            self.total = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for total: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<ToolRun> for super::ToolRun {
        type Error = super::error::ConversionError;
        fn try_from(value: ToolRun) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                segments: value.segments?,
                total: value.total?,
            })
        }
    }
    impl ::std::convert::From<super::ToolRun> for ToolRun {
        fn from(value: super::ToolRun) -> Self {
            Self {
                segments: Ok(value.segments),
                total: Ok(value.total),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ToolRunSegment {
        details: ::std::result::Result<super::EitherOrBothForSegmentDetails, ::std::string::String>,
        metrics_summary: ::std::result::Result<super::ToolMetricSummary, ::std::string::String>,
    }
    impl ::std::default::Default for ToolRunSegment {
        fn default() -> Self {
            Self {
                details: Err("no value supplied for details".to_string()),
                metrics_summary: Err("no value supplied for metrics_summary".to_string()),
            }
        }
    }
    impl ToolRunSegment {
        pub fn details<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::EitherOrBothForSegmentDetails>,
            T::Error: ::std::fmt::Display,
        {
            self.details = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for details: {}", e));
            self
        }
        pub fn metrics_summary<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ToolMetricSummary>,
            T::Error: ::std::fmt::Display,
        {
            self.metrics_summary = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for metrics_summary: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<ToolRunSegment> for super::ToolRunSegment {
        type Error = super::error::ConversionError;
        fn try_from(value: ToolRunSegment) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                details: value.details?,
                metrics_summary: value.metrics_summary?,
            })
        }
    }
    impl ::std::convert::From<super::ToolRunSegment> for ToolRunSegment {
        fn from(value: super::ToolRunSegment) -> Self {
            Self {
                details: Ok(value.details),
                metrics_summary: Ok(value.metrics_summary),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ToolSummary {
        log_paths: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        out_paths: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        summaries: ::std::result::Result<super::ToolRun, ::std::string::String>,
        tool: ::std::result::Result<super::ValgrindTool, ::std::string::String>,
    }
    impl ::std::default::Default for ToolSummary {
        fn default() -> Self {
            Self {
                log_paths: Err("no value supplied for log_paths".to_string()),
                out_paths: Err("no value supplied for out_paths".to_string()),
                summaries: Err("no value supplied for summaries".to_string()),
                tool: Err("no value supplied for tool".to_string()),
            }
        }
    }
    impl ToolSummary {
        pub fn log_paths<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.log_paths = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for log_paths: {}", e));
            self
        }
        pub fn out_paths<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<::std::string::String>>,
            T::Error: ::std::fmt::Display,
        {
            self.out_paths = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for out_paths: {}", e));
            self
        }
        pub fn summaries<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ToolRun>,
            T::Error: ::std::fmt::Display,
        {
            self.summaries = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for summaries: {}", e));
            self
        }
        pub fn tool<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ValgrindTool>,
            T::Error: ::std::fmt::Display,
        {
            self.tool = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for tool: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<ToolSummary> for super::ToolSummary {
        type Error = super::error::ConversionError;
        fn try_from(value: ToolSummary) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                log_paths: value.log_paths?,
                out_paths: value.out_paths?,
                summaries: value.summaries?,
                tool: value.tool?,
            })
        }
    }
    impl ::std::convert::From<super::ToolSummary> for ToolSummary {
        fn from(value: super::ToolSummary) -> Self {
            Self {
                log_paths: Ok(value.log_paths),
                out_paths: Ok(value.out_paths),
                summaries: Ok(value.summaries),
                tool: Ok(value.tool),
            }
        }
    }
}
