//! Generated using [cargo-typify](https://github.com/oxidecomputer/typify/tree/main/cargo-typify)
//! from [summary.v4.schema.json](https://github.com/iai-callgrind/iai-callgrind/blob/85845bbb16726ca7f9d0603388b2ec8f1ac8a357/iai-callgrind-runner/schemas/summary.v4.schema.json).
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
#[doc = "The `BenchmarkSummary` containing all the information of a single benchmark run\n\nThis includes produced files, recorded callgrind events, performance regressions ..."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"title\": \"BenchmarkSummary\","]
#[doc = "  \"description\": \"The `BenchmarkSummary` containing all the information of a single benchmark run\\n\\nThis includes produced files, recorded callgrind events, performance regressions ...\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"baselines\","]
#[doc = "    \"benchmark_exe\","]
#[doc = "    \"benchmark_file\","]
#[doc = "    \"function_name\","]
#[doc = "    \"kind\","]
#[doc = "    \"module_path\","]
#[doc = "    \"package_dir\","]
#[doc = "    \"profiles\","]
#[doc = "    \"project_root\","]
#[doc = "    \"version\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"baselines\": {"]
#[doc = "      \"description\": \"The baselines if any. An absent first baseline indicates that new output was produced. An\\nabsent second baseline indicates the usage of the usual \\\"*.old\\\" output.\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": ["]
#[doc = "        {"]
#[doc = "          \"type\": ["]
#[doc = "            \"string\","]
#[doc = "            \"null\""]
#[doc = "          ]"]
#[doc = "        },"]
#[doc = "        {"]
#[doc = "          \"type\": ["]
#[doc = "            \"string\","]
#[doc = "            \"null\""]
#[doc = "          ]"]
#[doc = "        }"]
#[doc = "      ],"]
#[doc = "      \"maxItems\": 2,"]
#[doc = "      \"minItems\": 2"]
#[doc = "    },"]
#[doc = "    \"benchmark_exe\": {"]
#[doc = "      \"description\": \"The path to the binary which is executed by valgrind. In case of a library benchmark this\\nis the compiled benchmark file. In case of a binary benchmark this is the path to the\\ncommand.\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"benchmark_file\": {"]
#[doc = "      \"description\": \"The path to the benchmark file\","]
#[doc = "      \"type\": \"string\""]
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
#[doc = "    \"profiles\": {"]
#[doc = "      \"description\": \"The summary of other valgrind tool runs\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/Profiles\""]
#[doc = "        }"]
#[doc = "      ]"]
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
#[doc = "    \"version\": {"]
#[doc = "      \"description\": \"The version of this format. Only backwards incompatible changes cause an increase of the\\nversion\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct BenchmarkSummary {
    #[doc = "The baselines if any. An absent first baseline indicates that new output was produced. An\nabsent second baseline indicates the usage of the usual \"*.old\" output."]
    pub baselines: (
        ::std::option::Option<::std::string::String>,
        ::std::option::Option<::std::string::String>,
    ),
    #[doc = "The path to the binary which is executed by valgrind. In case of a library benchmark this\nis the compiled benchmark file. In case of a binary benchmark this is the path to the\ncommand."]
    pub benchmark_exe: ::std::string::String,
    #[doc = "The path to the benchmark file"]
    pub benchmark_file: ::std::string::String,
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
    #[doc = "The summary of other valgrind tool runs"]
    pub profiles: Profiles,
    #[doc = "The project's root directory"]
    pub project_root: ::std::string::String,
    #[doc = "The destination and kind of the summary file"]
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub summary_output: ::std::option::Option<SummaryOutput>,
    #[doc = "The version of this format. Only backwards incompatible changes cause an increase of the\nversion"]
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
#[doc = "All metrics which cachegrind produces and additionally some derived events\n\nDepending on the options passed to Cachegrind, these are the events that Cachegrind can produce.\nSee the [Cachegrind\ndocumentation](https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts) for details."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"All metrics which cachegrind produces and additionally some derived events\\n\\nDepending on the options passed to Cachegrind, these are the events that Cachegrind can produce.\\nSee the [Cachegrind\\ndocumentation](https://valgrind.org/docs/manual/cg-manual.html#cg-manual.cgopts) for details.\","]
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
#[doc = "      \"description\": \"I1 cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"I1MissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 instructions cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLiMissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"D1 cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"D1MissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 data cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLdMissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLMissRate\""]
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
#[doc = "      \"description\": \"L1 cache hit rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"L1HitRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 cache hit rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLHitRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"RAM hit rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"RamHitRate\""]
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
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CachegrindMetric {
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
    #[doc = "I1 cache miss rate (--cache-sim=yes)"]
    I1MissRate,
    #[doc = "LL/L2 instructions cache miss rate (--cache-sim=yes)"]
    LLiMissRate,
    #[doc = "D1 cache miss rate (--cache-sim=yes)"]
    D1MissRate,
    #[doc = "LL/L2 data cache miss rate (--cache-sim=yes)"]
    LLdMissRate,
    #[doc = "LL/L2 cache miss rate (--cache-sim=yes)"]
    #[serde(rename = "LLMissRate")]
    LlMissRate,
    #[doc = "Derived event showing the L1 hits (--cache-sim=yes)"]
    L1hits,
    #[doc = "Derived event showing the LL hits (--cache-sim=yes)"]
    LLhits,
    #[doc = "Derived event showing the RAM hits (--cache-sim=yes)"]
    RamHits,
    #[doc = "L1 cache hit rate (--cache-sim=yes)"]
    L1HitRate,
    #[doc = "LL/L2 cache hit rate (--cache-sim=yes)"]
    #[serde(rename = "LLHitRate")]
    LlHitRate,
    #[doc = "RAM hit rate (--cache-sim=yes)"]
    RamHitRate,
    #[doc = "Derived event showing the total amount of cache reads and writes (--cache-sim=yes)"]
    #[serde(rename = "TotalRW")]
    TotalRw,
    #[doc = "Derived event showing estimated CPU cycles (--cache-sim=yes)"]
    EstimatedCycles,
    #[doc = "Conditional branches executed (--branch-sim=yes)"]
    Bc,
    #[doc = "Conditional branches mispredicted (--branch-sim=yes)"]
    Bcm,
    #[doc = "Indirect branches executed (--branch-sim=yes)"]
    Bi,
    #[doc = "Indirect branches mispredicted (--branch-sim=yes)"]
    Bim,
}
impl ::std::convert::From<&Self> for CachegrindMetric {
    fn from(value: &CachegrindMetric) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for CachegrindMetric {
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
            Self::I1MissRate => write!(f, "I1MissRate"),
            Self::LLiMissRate => write!(f, "LLiMissRate"),
            Self::D1MissRate => write!(f, "D1MissRate"),
            Self::LLdMissRate => write!(f, "LLdMissRate"),
            Self::LlMissRate => write!(f, "LLMissRate"),
            Self::L1hits => write!(f, "L1hits"),
            Self::LLhits => write!(f, "LLhits"),
            Self::RamHits => write!(f, "RamHits"),
            Self::L1HitRate => write!(f, "L1HitRate"),
            Self::LlHitRate => write!(f, "LLHitRate"),
            Self::RamHitRate => write!(f, "RamHitRate"),
            Self::TotalRw => write!(f, "TotalRW"),
            Self::EstimatedCycles => write!(f, "EstimatedCycles"),
            Self::Bc => write!(f, "Bc"),
            Self::Bcm => write!(f, "Bcm"),
            Self::Bi => write!(f, "Bi"),
            Self::Bim => write!(f, "Bim"),
        }
    }
}
impl ::std::str::FromStr for CachegrindMetric {
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
            "I1MissRate" => Ok(Self::I1MissRate),
            "LLiMissRate" => Ok(Self::LLiMissRate),
            "D1MissRate" => Ok(Self::D1MissRate),
            "LLdMissRate" => Ok(Self::LLdMissRate),
            "LLMissRate" => Ok(Self::LlMissRate),
            "L1hits" => Ok(Self::L1hits),
            "LLhits" => Ok(Self::LLhits),
            "RamHits" => Ok(Self::RamHits),
            "L1HitRate" => Ok(Self::L1HitRate),
            "LLHitRate" => Ok(Self::LlHitRate),
            "RamHitRate" => Ok(Self::RamHitRate),
            "TotalRW" => Ok(Self::TotalRw),
            "EstimatedCycles" => Ok(Self::EstimatedCycles),
            "Bc" => Ok(Self::Bc),
            "Bcm" => Ok(Self::Bcm),
            "Bi" => Ok(Self::Bi),
            "Bim" => Ok(Self::Bim),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for CachegrindMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for CachegrindMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for CachegrindMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "The metrics collected by DHAT"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The metrics collected by DHAT\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"In ad-hoc mode, Total units measured over the entire execution\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"TotalUnits\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Total ad-hoc events over the entire execution\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"TotalEvents\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Total bytes allocated over the entire execution\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"TotalBytes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"Total heap blocks allocated over the entire execution\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"TotalBlocks\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The bytes alive at t-gmax, the time when the heap size reached its global maximum\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"AtTGmaxBytes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The blocks alive at t-gmax\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"AtTGmaxBlocks\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of bytes at the end of the execution.\\n\\nThis is the amount of bytes which were not explicitly freed.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"AtTEndBytes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of blocks at the end of the execution.\\n\\nThis is the amount of heap blocks which were not explicitly freed.\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"AtTEndBlocks\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of bytes read during the entire execution\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"ReadsBytes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of bytes written during the entire execution\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"WritesBytes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The total lifetimes of all heap blocks allocated\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"TotalLifetimes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The maximum amount of bytes\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"MaximumBytes\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The maximum amount of heap blocks\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"MaximumBlocks\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DhatMetric {
    #[doc = "In ad-hoc mode, Total units measured over the entire execution"]
    TotalUnits,
    #[doc = "Total ad-hoc events over the entire execution"]
    TotalEvents,
    #[doc = "Total bytes allocated over the entire execution"]
    TotalBytes,
    #[doc = "Total heap blocks allocated over the entire execution"]
    TotalBlocks,
    #[doc = "The bytes alive at t-gmax, the time when the heap size reached its global maximum"]
    AtTGmaxBytes,
    #[doc = "The blocks alive at t-gmax"]
    AtTGmaxBlocks,
    #[doc = "The amount of bytes at the end of the execution.\n\nThis is the amount of bytes which were not explicitly freed."]
    AtTEndBytes,
    #[doc = "The amount of blocks at the end of the execution.\n\nThis is the amount of heap blocks which were not explicitly freed."]
    AtTEndBlocks,
    #[doc = "The amount of bytes read during the entire execution"]
    ReadsBytes,
    #[doc = "The amount of bytes written during the entire execution"]
    WritesBytes,
    #[doc = "The total lifetimes of all heap blocks allocated"]
    TotalLifetimes,
    #[doc = "The maximum amount of bytes"]
    MaximumBytes,
    #[doc = "The maximum amount of heap blocks"]
    MaximumBlocks,
}
impl ::std::convert::From<&Self> for DhatMetric {
    fn from(value: &DhatMetric) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for DhatMetric {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::TotalUnits => write!(f, "TotalUnits"),
            Self::TotalEvents => write!(f, "TotalEvents"),
            Self::TotalBytes => write!(f, "TotalBytes"),
            Self::TotalBlocks => write!(f, "TotalBlocks"),
            Self::AtTGmaxBytes => write!(f, "AtTGmaxBytes"),
            Self::AtTGmaxBlocks => write!(f, "AtTGmaxBlocks"),
            Self::AtTEndBytes => write!(f, "AtTEndBytes"),
            Self::AtTEndBlocks => write!(f, "AtTEndBlocks"),
            Self::ReadsBytes => write!(f, "ReadsBytes"),
            Self::WritesBytes => write!(f, "WritesBytes"),
            Self::TotalLifetimes => write!(f, "TotalLifetimes"),
            Self::MaximumBytes => write!(f, "MaximumBytes"),
            Self::MaximumBlocks => write!(f, "MaximumBlocks"),
        }
    }
}
impl ::std::str::FromStr for DhatMetric {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "TotalUnits" => Ok(Self::TotalUnits),
            "TotalEvents" => Ok(Self::TotalEvents),
            "TotalBytes" => Ok(Self::TotalBytes),
            "TotalBlocks" => Ok(Self::TotalBlocks),
            "AtTGmaxBytes" => Ok(Self::AtTGmaxBytes),
            "AtTGmaxBlocks" => Ok(Self::AtTGmaxBlocks),
            "AtTEndBytes" => Ok(Self::AtTEndBytes),
            "AtTEndBlocks" => Ok(Self::AtTEndBlocks),
            "ReadsBytes" => Ok(Self::ReadsBytes),
            "WritesBytes" => Ok(Self::WritesBytes),
            "TotalLifetimes" => Ok(Self::TotalLifetimes),
            "MaximumBytes" => Ok(Self::MaximumBytes),
            "MaximumBlocks" => Ok(Self::MaximumBlocks),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for DhatMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DhatMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DhatMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
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
#[doc = "      \"description\": \"The percentage of the difference between two `Metrics` serialized as string to preserve\\ninfinity values and avoid `null` in json\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    },"]
#[doc = "    \"factor\": {"]
#[doc = "      \"description\": \"The factor of the difference between two `Metrics` serialized as string to preserve\\ninfinity values and void `null` in json\","]
#[doc = "      \"type\": \"string\""]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct Diffs {
    #[doc = "The percentage of the difference between two `Metrics` serialized as string to preserve\ninfinity values and avoid `null` in json"]
    pub diff_pct: ::std::string::String,
    #[doc = "The factor of the difference between two `Metrics` serialized as string to preserve\ninfinity values and void `null` in json"]
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
#[doc = "Either left or right or both can be present\n\nMost of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\nleft is `new` and right is `old`."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Either left or right or both can be present\\n\\nMost of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\\nleft is `new` and right is `old`.\","]
#[doc = "  \"oneOf\": ["]
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
#[doc = "              \"$ref\": \"#/definitions/ProfileInfo\""]
#[doc = "            },"]
#[doc = "            {"]
#[doc = "              \"$ref\": \"#/definitions/ProfileInfo\""]
#[doc = "            }"]
#[doc = "          ],"]
#[doc = "          \"maxItems\": 2,"]
#[doc = "          \"minItems\": 2"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The left or `new` value\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Left\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Left\": {"]
#[doc = "          \"$ref\": \"#/definitions/ProfileInfo\""]
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
#[doc = "          \"$ref\": \"#/definitions/ProfileInfo\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum EitherOrBoth {
    #[doc = "Both values (`new` and `old`) are present"]
    Both(ProfileInfo, ProfileInfo),
    #[doc = "The left or `new` value"]
    Left(ProfileInfo),
    #[doc = "The right or `old` value"]
    Right(ProfileInfo),
}
impl ::std::convert::From<&Self> for EitherOrBoth {
    fn from(value: &EitherOrBoth) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<(ProfileInfo, ProfileInfo)> for EitherOrBoth {
    fn from(value: (ProfileInfo, ProfileInfo)) -> Self {
        Self::Both(value.0, value.1)
    }
}
#[doc = "Either left or right or both can be present\n\nMost of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\nleft is `new` and right is `old`."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"Either left or right or both can be present\\n\\nMost of the time, this enum is used to store (new, old) output, metrics, etc. Per convention\\nleft is `new` and right is `old`.\","]
#[doc = "  \"oneOf\": ["]
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
#[doc = "              \"$ref\": \"#/definitions/Metric\""]
#[doc = "            },"]
#[doc = "            {"]
#[doc = "              \"$ref\": \"#/definitions/Metric\""]
#[doc = "            }"]
#[doc = "          ],"]
#[doc = "          \"maxItems\": 2,"]
#[doc = "          \"minItems\": 2"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The left or `new` value\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Left\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Left\": {"]
#[doc = "          \"$ref\": \"#/definitions/Metric\""]
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
#[doc = "          \"$ref\": \"#/definitions/Metric\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum EitherOrBoth2 {
    #[doc = "Both values (`new` and `old`) are present"]
    Both(Metric, Metric),
    #[doc = "The left or `new` value"]
    Left(Metric),
    #[doc = "The right or `old` value"]
    Right(Metric),
}
impl ::std::convert::From<&Self> for EitherOrBoth2 {
    fn from(value: &EitherOrBoth2) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<(Metric, Metric)> for EitherOrBoth2 {
    fn from(value: (Metric, Metric)) -> Self {
        Self::Both(value.0, value.1)
    }
}
#[doc = "The error metrics from a tool which reports errors\n\nThe tools which report only errors are `helgrind`, `drd` and `memcheck`. The order in which the\nvariants are defined in this enum determines the order of the metrics in the benchmark terminal\noutput."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The error metrics from a tool which reports errors\\n\\nThe tools which report only errors are `helgrind`, `drd` and `memcheck`. The order in which the\\nvariants are defined in this enum determines the order of the metrics in the benchmark terminal\\noutput.\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of detected unsuppressed errors\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Errors\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of detected unsuppressed error contexts\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Contexts\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of suppressed errors\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SuppressedErrors\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The amount of suppressed error contexts\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"SuppressedContexts\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorMetric {
    #[doc = "The amount of detected unsuppressed errors"]
    Errors,
    #[doc = "The amount of detected unsuppressed error contexts"]
    Contexts,
    #[doc = "The amount of suppressed errors"]
    SuppressedErrors,
    #[doc = "The amount of suppressed error contexts"]
    SuppressedContexts,
}
impl ::std::convert::From<&Self> for ErrorMetric {
    fn from(value: &ErrorMetric) -> Self {
        value.clone()
    }
}
impl ::std::fmt::Display for ErrorMetric {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Errors => write!(f, "Errors"),
            Self::Contexts => write!(f, "Contexts"),
            Self::SuppressedErrors => write!(f, "SuppressedErrors"),
            Self::SuppressedContexts => write!(f, "SuppressedContexts"),
        }
    }
}
impl ::std::str::FromStr for ErrorMetric {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "Errors" => Ok(Self::Errors),
            "Contexts" => Ok(Self::Contexts),
            "SuppressedErrors" => Ok(Self::SuppressedErrors),
            "SuppressedContexts" => Ok(Self::SuppressedContexts),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ErrorMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ErrorMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ErrorMetric {
    type Error = self::error::ConversionError;
    fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, self::error::ConversionError> {
        value.parse()
    }
}
#[doc = "All `EventKind`s callgrind produces and additionally some derived events\n\nDepending on the options passed to Callgrind, these are the events that Callgrind can produce.\nSee the [Callgrind\ndocumentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"All `EventKind`s callgrind produces and additionally some derived events\\n\\nDepending on the options passed to Callgrind, these are the events that Callgrind can produce.\\nSee the [Callgrind\\ndocumentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details.\","]
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
#[doc = "      \"description\": \"I1 cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"I1MissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 instructions cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLiMissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"D1 cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"D1MissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 data cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLdMissRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 cache miss rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLMissRate\""]
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
#[doc = "      \"description\": \"L1 cache hit rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"L1HitRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"LL/L2 cache hit rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"LLHitRate\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"RAM hit rate (--cache-sim=yes)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"RamHitRate\""]
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
    #[doc = "I1 cache miss rate (--cache-sim=yes)"]
    I1MissRate,
    #[doc = "LL/L2 instructions cache miss rate (--cache-sim=yes)"]
    LLiMissRate,
    #[doc = "D1 cache miss rate (--cache-sim=yes)"]
    D1MissRate,
    #[doc = "LL/L2 data cache miss rate (--cache-sim=yes)"]
    LLdMissRate,
    #[doc = "LL/L2 cache miss rate (--cache-sim=yes)"]
    #[serde(rename = "LLMissRate")]
    LlMissRate,
    #[doc = "Derived event showing the L1 hits (--cache-sim=yes)"]
    L1hits,
    #[doc = "Derived event showing the LL hits (--cache-sim=yes)"]
    LLhits,
    #[doc = "Derived event showing the RAM hits (--cache-sim=yes)"]
    RamHits,
    #[doc = "L1 cache hit rate (--cache-sim=yes)"]
    L1HitRate,
    #[doc = "LL/L2 cache hit rate (--cache-sim=yes)"]
    #[serde(rename = "LLHitRate")]
    LlHitRate,
    #[doc = "RAM hit rate (--cache-sim=yes)"]
    RamHitRate,
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
            Self::I1MissRate => write!(f, "I1MissRate"),
            Self::LLiMissRate => write!(f, "LLiMissRate"),
            Self::D1MissRate => write!(f, "D1MissRate"),
            Self::LLdMissRate => write!(f, "LLdMissRate"),
            Self::LlMissRate => write!(f, "LLMissRate"),
            Self::L1hits => write!(f, "L1hits"),
            Self::LLhits => write!(f, "LLhits"),
            Self::RamHits => write!(f, "RamHits"),
            Self::L1HitRate => write!(f, "L1HitRate"),
            Self::LlHitRate => write!(f, "LLHitRate"),
            Self::RamHitRate => write!(f, "RamHitRate"),
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
            "I1MissRate" => Ok(Self::I1MissRate),
            "LLiMissRate" => Ok(Self::LLiMissRate),
            "D1MissRate" => Ok(Self::D1MissRate),
            "LLdMissRate" => Ok(Self::LLdMissRate),
            "LLMissRate" => Ok(Self::LlMissRate),
            "L1hits" => Ok(Self::L1hits),
            "LLhits" => Ok(Self::LLhits),
            "RamHits" => Ok(Self::RamHits),
            "L1HitRate" => Ok(Self::L1HitRate),
            "LLHitRate" => Ok(Self::LlHitRate),
            "RamHitRate" => Ok(Self::RamHitRate),
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
#[doc = "The callgrind `FlamegraphSummary` records all created paths for an [`EventKind`] specific\nflamegraph\n\nEither the `regular_path`, `old_path` or the `diff_path` are present. Never can all of them be\nabsent."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The callgrind `FlamegraphSummary` records all created paths for an [`EventKind`] specific\\nflamegraph\\n\\nEither the `regular_path`, `old_path` or the `diff_path` are present. Never can all of them be\\nabsent.\","]
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
#[doc = "The metric measured by valgrind or derived from one or more other metrics\n\nThe valgrind metrics measured by any of its tools are `u64`. However, to be able to represent\nderived metrics like cache miss/hit rates it is inevitable to have a type which can store a\n`u64` or a `f64`. When doing math with metrics, the original type should be preserved as far as\npossible by using `u64` operations. A float metric should be a last resort.\n\nFloat operations with a `Metric` that stores a `u64` introduce a precision loss and are to be\navoided. Especially comparison between a `u64` metric and `f64` metric are not exact because the\n`u64` has to be converted to a `f64`. Also, if adding/multiplying two `u64` metrics would result\nin an overflow the metric saturates at `u64::MAX`. This choice was made to preserve precision\nand the original type (instead of for example adding the two `u64` by converting both of them to\n`f64`)."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The metric measured by valgrind or derived from one or more other metrics\\n\\nThe valgrind metrics measured by any of its tools are `u64`. However, to be able to represent\\nderived metrics like cache miss/hit rates it is inevitable to have a type which can store a\\n`u64` or a `f64`. When doing math with metrics, the original type should be preserved as far as\\npossible by using `u64` operations. A float metric should be a last resort.\\n\\nFloat operations with a `Metric` that stores a `u64` introduce a precision loss and are to be\\navoided. Especially comparison between a `u64` metric and `f64` metric are not exact because the\\n`u64` has to be converted to a `f64`. Also, if adding/multiplying two `u64` metrics would result\\nin an overflow the metric saturates at `u64::MAX`. This choice was made to preserve precision\\nand the original type (instead of for example adding the two `u64` by converting both of them to\\n`f64`).\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"An integer `Metric`\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Int\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Int\": {"]
#[doc = "          \"type\": \"integer\","]
#[doc = "          \"format\": \"uint64\","]
#[doc = "          \"minimum\": 0.0"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"A float `Metric`\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Float\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Float\": {"]
#[doc = "          \"type\": \"number\","]
#[doc = "          \"format\": \"double\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum Metric {
    #[doc = "An integer `Metric`"]
    Int(u64),
    #[doc = "A float `Metric`"]
    Float(f64),
}
impl ::std::convert::From<&Self> for Metric {
    fn from(value: &Metric) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<u64> for Metric {
    fn from(value: u64) -> Self {
        Self::Int(value)
    }
}
impl ::std::convert::From<f64> for Metric {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}
#[doc = "The different metrics distinguished by tool and if it is an error checking tool as `ErrorMetric`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The different metrics distinguished by tool and if it is an error checking tool as `ErrorMetric`\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"The `None` kind if there are no metrics for a tool\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"None\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The Callgrind metric kind\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Callgrind\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Callgrind\": {"]
#[doc = "          \"$ref\": \"#/definitions/EventKind\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The Cachegrind metric kind\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Cachegrind\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Cachegrind\": {"]
#[doc = "          \"$ref\": \"#/definitions/CachegrindMetric\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The DHAT metric kind\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Dhat\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Dhat\": {"]
#[doc = "          \"$ref\": \"#/definitions/DhatMetric\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The Memcheck metric kind\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Memcheck\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Memcheck\": {"]
#[doc = "          \"$ref\": \"#/definitions/ErrorMetric\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The Helgrind metric kind\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Helgrind\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Helgrind\": {"]
#[doc = "          \"$ref\": \"#/definitions/ErrorMetric\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The DRD metric kind\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"DRD\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"DRD\": {"]
#[doc = "          \"$ref\": \"#/definitions/ErrorMetric\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum MetricKind {
    #[doc = "The `None` kind if there are no metrics for a tool"]
    None,
    #[doc = "The Callgrind metric kind"]
    Callgrind(EventKind),
    #[doc = "The Cachegrind metric kind"]
    Cachegrind(CachegrindMetric),
    #[doc = "The DHAT metric kind"]
    Dhat(DhatMetric),
    #[doc = "The Memcheck metric kind"]
    Memcheck(ErrorMetric),
    #[doc = "The Helgrind metric kind"]
    Helgrind(ErrorMetric),
    #[doc = "The DRD metric kind"]
    #[serde(rename = "DRD")]
    Drd(ErrorMetric),
}
impl ::std::convert::From<&Self> for MetricKind {
    fn from(value: &MetricKind) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<EventKind> for MetricKind {
    fn from(value: EventKind) -> Self {
        Self::Callgrind(value)
    }
}
impl ::std::convert::From<CachegrindMetric> for MetricKind {
    fn from(value: CachegrindMetric) -> Self {
        Self::Cachegrind(value)
    }
}
impl ::std::convert::From<DhatMetric> for MetricKind {
    fn from(value: DhatMetric) -> Self {
        Self::Dhat(value)
    }
}
#[doc = "The `MetricsDiff` describes the difference between a `new` and `old` metric as percentage and\nfactor.\n\nOnly if both metrics are present there is also a `Diffs` present. Otherwise, it just stores the\n`new` or `old` metric."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `MetricsDiff` describes the difference between a `new` and `old` metric as percentage and\\nfactor.\\n\\nOnly if both metrics are present there is also a `Diffs` present. Otherwise, it just stores the\\n`new` or `old` metric.\","]
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
#[doc = "          \"$ref\": \"#/definitions/EitherOrBoth2\""]
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
    pub metrics: EitherOrBoth2,
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
pub struct MetricsSummary(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummary {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummary> for ::std::collections::HashMap<::std::string::String, MetricsDiff> {
    fn from(value: MetricsSummary) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummary> for MetricsSummary {
    fn from(value: &MetricsSummary) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>> for MetricsSummary {
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
pub struct MetricsSummary2(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummary2 {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummary2> for ::std::collections::HashMap<::std::string::String, MetricsDiff> {
    fn from(value: MetricsSummary2) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummary2> for MetricsSummary2 {
    fn from(value: &MetricsSummary2) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>> for MetricsSummary2 {
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
pub struct MetricsSummary3(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummary3 {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummary3> for ::std::collections::HashMap<::std::string::String, MetricsDiff> {
    fn from(value: MetricsSummary3) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummary3> for MetricsSummary3 {
    fn from(value: &MetricsSummary3) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>> for MetricsSummary3 {
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
pub struct MetricsSummary4(pub ::std::collections::HashMap<::std::string::String, MetricsDiff>);
impl ::std::ops::Deref for MetricsSummary4 {
    type Target = ::std::collections::HashMap<::std::string::String, MetricsDiff>;
    fn deref(&self) -> &::std::collections::HashMap<::std::string::String, MetricsDiff> {
        &self.0
    }
}
impl ::std::convert::From<MetricsSummary4> for ::std::collections::HashMap<::std::string::String, MetricsDiff> {
    fn from(value: MetricsSummary4) -> Self {
        value.0
    }
}
impl ::std::convert::From<&MetricsSummary4> for MetricsSummary4 {
    fn from(value: &MetricsSummary4) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::collections::HashMap<::std::string::String, MetricsDiff>> for MetricsSummary4 {
    fn from(value: ::std::collections::HashMap<::std::string::String, MetricsDiff>) -> Self {
        Self(value)
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
#[doc = "    \"flamegraphs\","]
#[doc = "    \"log_paths\","]
#[doc = "    \"out_paths\","]
#[doc = "    \"summaries\","]
#[doc = "    \"tool\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"flamegraphs\": {"]
#[doc = "      \"description\": \"Details and information about the created flamegraphs if any\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/FlamegraphSummary\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"log_paths\": {"]
#[doc = "      \"description\": \"The paths to the `*.log` files. All tools produce at least one log file\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"out_paths\": {"]
#[doc = "      \"description\": \"The paths to the `*.out` files. Not all tools produce an output in addition to the log\\nfiles\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"type\": \"string\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"summaries\": {"]
#[doc = "      \"description\": \"The metrics and details about the tool run\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/ProfileData\""]
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
pub struct Profile {
    #[doc = "Details and information about the created flamegraphs if any"]
    pub flamegraphs: ::std::vec::Vec<FlamegraphSummary>,
    #[doc = "The paths to the `*.log` files. All tools produce at least one log file"]
    pub log_paths: ::std::vec::Vec<::std::string::String>,
    #[doc = "The paths to the `*.out` files. Not all tools produce an output in addition to the log\nfiles"]
    pub out_paths: ::std::vec::Vec<::std::string::String>,
    #[doc = "The metrics and details about the tool run"]
    pub summaries: ProfileData,
    #[doc = "The Valgrind tool like `DHAT`, `Memcheck` etc."]
    pub tool: ValgrindTool,
}
impl ::std::convert::From<&Profile> for Profile {
    fn from(value: &Profile) -> Self {
        value.clone()
    }
}
impl Profile {
    pub fn builder() -> builder::Profile {
        Default::default()
    }
}
#[doc = "The `ToolRun` contains all information about a single tool run with possibly multiple segments\n\nThe total is always present and summarizes all tool run segments. In the special case of a\nsingle tool run segment, the total equals the metrics of this segment."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The `ToolRun` contains all information about a single tool run with possibly multiple segments\\n\\nThe total is always present and summarizes all tool run segments. In the special case of a\\nsingle tool run segment, the total equals the metrics of this segment.\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"parts\","]
#[doc = "    \"total\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"parts\": {"]
#[doc = "      \"description\": \"All [`ProfilePart`]s\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/ProfilePart\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"total\": {"]
#[doc = "      \"description\": \"The total over the [`ProfilePart`]s\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/ProfileTotal\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    }"]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub struct ProfileData {
    #[doc = "All [`ProfilePart`]s"]
    pub parts: ::std::vec::Vec<ProfilePart>,
    #[doc = "The total over the [`ProfilePart`]s"]
    pub total: ProfileTotal,
}
impl ::std::convert::From<&ProfileData> for ProfileData {
    fn from(value: &ProfileData) -> Self {
        value.clone()
    }
}
impl ProfileData {
    pub fn builder() -> builder::ProfileData {
        Default::default()
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
pub struct ProfileInfo {
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
impl ::std::convert::From<&ProfileInfo> for ProfileInfo {
    fn from(value: &ProfileInfo) -> Self {
        value.clone()
    }
}
impl ProfileInfo {
    pub fn builder() -> builder::ProfileInfo {
        Default::default()
    }
}
#[doc = "A single segment of a tool run and if present the comparison with the \"old\" segment\n\nA tool run can produce multiple segments, for example for each process and subprocess with\n(--trace-children)."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A single segment of a tool run and if present the comparison with the \\\"old\\\" segment\\n\\nA tool run can produce multiple segments, for example for each process and subprocess with\\n(--trace-children).\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"details\","]
#[doc = "    \"metrics_summary\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"details\": {"]
#[doc = "      \"description\": \"Details like command, pid, ppid, thread number etc. (see [`ProfileInfo`])\","]
#[doc = "      \"allOf\": ["]
#[doc = "        {"]
#[doc = "          \"$ref\": \"#/definitions/EitherOrBoth\""]
#[doc = "        }"]
#[doc = "      ]"]
#[doc = "    },"]
#[doc = "    \"metrics_summary\": {"]
#[doc = "      \"description\": \"The [`ToolMetricSummary`]\","]
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
pub struct ProfilePart {
    #[doc = "Details like command, pid, ppid, thread number etc. (see [`ProfileInfo`])"]
    pub details: EitherOrBoth,
    #[doc = "The [`ToolMetricSummary`]"]
    pub metrics_summary: ToolMetricSummary,
}
impl ::std::convert::From<&ProfilePart> for ProfilePart {
    fn from(value: &ProfilePart) -> Self {
        value.clone()
    }
}
impl ProfilePart {
    pub fn builder() -> builder::ProfilePart {
        Default::default()
    }
}
#[doc = "The total metrics over all [`ProfilePart`]s and if detected any [`ToolRegression`]"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The total metrics over all [`ProfilePart`]s and if detected any [`ToolRegression`]\","]
#[doc = "  \"type\": \"object\","]
#[doc = "  \"required\": ["]
#[doc = "    \"regressions\","]
#[doc = "    \"summary\""]
#[doc = "  ],"]
#[doc = "  \"properties\": {"]
#[doc = "    \"regressions\": {"]
#[doc = "      \"description\": \"The detected regressions if any\","]
#[doc = "      \"type\": \"array\","]
#[doc = "      \"items\": {"]
#[doc = "        \"$ref\": \"#/definitions/ToolRegression\""]
#[doc = "      }"]
#[doc = "    },"]
#[doc = "    \"summary\": {"]
#[doc = "      \"description\": \"The summary of metrics of the tool\","]
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
pub struct ProfileTotal {
    #[doc = "The detected regressions if any"]
    pub regressions: ::std::vec::Vec<ToolRegression>,
    #[doc = "The summary of metrics of the tool"]
    pub summary: ToolMetricSummary,
}
impl ::std::convert::From<&ProfileTotal> for ProfileTotal {
    fn from(value: &ProfileTotal) -> Self {
        value.clone()
    }
}
impl ProfileTotal {
    pub fn builder() -> builder::ProfileTotal {
        Default::default()
    }
}
#[doc = "The collection of all generated [`Profile`]s"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The collection of all generated [`Profile`]s\","]
#[doc = "  \"type\": \"array\","]
#[doc = "  \"items\": {"]
#[doc = "    \"$ref\": \"#/definitions/Profile\""]
#[doc = "  }"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct Profiles(pub ::std::vec::Vec<Profile>);
impl ::std::ops::Deref for Profiles {
    type Target = ::std::vec::Vec<Profile>;
    fn deref(&self) -> &::std::vec::Vec<Profile> {
        &self.0
    }
}
impl ::std::convert::From<Profiles> for ::std::vec::Vec<Profile> {
    fn from(value: Profiles) -> Self {
        value.0
    }
}
impl ::std::convert::From<&Profiles> for Profiles {
    fn from(value: &Profiles) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<::std::vec::Vec<Profile>> for Profiles {
    fn from(value: ::std::vec::Vec<Profile>) -> Self {
        Self(value)
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
#[doc = "        \"ErrorTool\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"ErrorTool\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The dhat summary\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Dhat\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Dhat\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary2\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The callgrind summary\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Callgrind\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Callgrind\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary3\""]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"The cachegrind summary\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Cachegrind\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Cachegrind\": {"]
#[doc = "          \"$ref\": \"#/definitions/MetricsSummary4\""]
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
    ErrorTool(MetricsSummary),
    #[doc = "The dhat summary"]
    Dhat(MetricsSummary2),
    #[doc = "The callgrind summary"]
    Callgrind(MetricsSummary3),
    #[doc = "The cachegrind summary"]
    Cachegrind(MetricsSummary4),
}
impl ::std::convert::From<&Self> for ToolMetricSummary {
    fn from(value: &ToolMetricSummary) -> Self {
        value.clone()
    }
}
impl ::std::convert::From<MetricsSummary> for ToolMetricSummary {
    fn from(value: MetricsSummary) -> Self {
        Self::ErrorTool(value)
    }
}
impl ::std::convert::From<MetricsSummary2> for ToolMetricSummary {
    fn from(value: MetricsSummary2) -> Self {
        Self::Dhat(value)
    }
}
impl ::std::convert::From<MetricsSummary3> for ToolMetricSummary {
    fn from(value: MetricsSummary3) -> Self {
        Self::Callgrind(value)
    }
}
impl ::std::convert::From<MetricsSummary4> for ToolMetricSummary {
    fn from(value: MetricsSummary4) -> Self {
        Self::Cachegrind(value)
    }
}
#[doc = "A detected performance regression depending on the limit either `Soft` or `Hard`"]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"A detected performance regression depending on the limit either `Soft` or `Hard`\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"A performance regression triggered by a soft limit\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Soft\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Soft\": {"]
#[doc = "          \"type\": \"object\","]
#[doc = "          \"required\": ["]
#[doc = "            \"diff_pct\","]
#[doc = "            \"limit\","]
#[doc = "            \"metric\","]
#[doc = "            \"new\","]
#[doc = "            \"old\""]
#[doc = "          ],"]
#[doc = "          \"properties\": {"]
#[doc = "            \"diff_pct\": {"]
#[doc = "              \"description\": \"The difference between new and old in percent. Serialized as string to preserve\\ninfinity values and avoid null in json.\","]
#[doc = "              \"type\": \"string\""]
#[doc = "            },"]
#[doc = "            \"limit\": {"]
#[doc = "              \"description\": \"The value of the limit which was exceeded to cause a performance regression. Serialized\\nas string to preserve infinity values and avoid null in json.\","]
#[doc = "              \"type\": \"string\""]
#[doc = "            },"]
#[doc = "            \"metric\": {"]
#[doc = "              \"description\": \"The metric kind per tool\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/MetricKind\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            },"]
#[doc = "            \"new\": {"]
#[doc = "              \"description\": \"The value of the new benchmark run\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/Metric\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            },"]
#[doc = "            \"old\": {"]
#[doc = "              \"description\": \"The value of the old benchmark run\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/Metric\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            }"]
#[doc = "          }"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"A performance regression triggered by a hard limit\","]
#[doc = "      \"type\": \"object\","]
#[doc = "      \"required\": ["]
#[doc = "        \"Hard\""]
#[doc = "      ],"]
#[doc = "      \"properties\": {"]
#[doc = "        \"Hard\": {"]
#[doc = "          \"type\": \"object\","]
#[doc = "          \"required\": ["]
#[doc = "            \"diff\","]
#[doc = "            \"limit\","]
#[doc = "            \"metric\","]
#[doc = "            \"new\""]
#[doc = "          ],"]
#[doc = "          \"properties\": {"]
#[doc = "            \"diff\": {"]
#[doc = "              \"description\": \"The difference between new and the limit\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/Metric\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            },"]
#[doc = "            \"limit\": {"]
#[doc = "              \"description\": \"The limit\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/Metric\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            },"]
#[doc = "            \"metric\": {"]
#[doc = "              \"description\": \"The metric kind per tool\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/MetricKind\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            },"]
#[doc = "            \"new\": {"]
#[doc = "              \"description\": \"The value of the benchmark run\","]
#[doc = "              \"allOf\": ["]
#[doc = "                {"]
#[doc = "                  \"$ref\": \"#/definitions/Metric\""]
#[doc = "                }"]
#[doc = "              ]"]
#[doc = "            }"]
#[doc = "          }"]
#[doc = "        }"]
#[doc = "      },"]
#[doc = "      \"additionalProperties\": false"]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Debug)]
pub enum ToolRegression {
    #[doc = "A performance regression triggered by a soft limit"]
    Soft {
        #[doc = "The difference between new and old in percent. Serialized as string to preserve\ninfinity values and avoid null in json."]
        diff_pct: ::std::string::String,
        #[doc = "The value of the limit which was exceeded to cause a performance regression. Serialized\nas string to preserve infinity values and avoid null in json."]
        limit: ::std::string::String,
        #[doc = "The metric kind per tool"]
        metric: MetricKind,
        #[doc = "The value of the new benchmark run"]
        new: Metric,
        #[doc = "The value of the old benchmark run"]
        old: Metric,
    },
    #[doc = "A performance regression triggered by a hard limit"]
    Hard {
        #[doc = "The difference between new and the limit"]
        diff: Metric,
        #[doc = "The limit"]
        limit: Metric,
        #[doc = "The metric kind per tool"]
        metric: MetricKind,
        #[doc = "The value of the benchmark run"]
        new: Metric,
    },
}
impl ::std::convert::From<&Self> for ToolRegression {
    fn from(value: &ToolRegression) -> Self {
        value.clone()
    }
}
#[doc = "The valgrind tools which can be run\n\nNote the default changes from `Callgrind` to `Cachegrind` if the `cachegrind` feature is\nselected."]
#[doc = r""]
#[doc = r" <details><summary>JSON schema</summary>"]
#[doc = r""]
#[doc = r" ```json"]
#[doc = "{"]
#[doc = "  \"description\": \"The valgrind tools which can be run\\n\\nNote the default changes from `Callgrind` to `Cachegrind` if the `cachegrind` feature is\\nselected.\","]
#[doc = "  \"oneOf\": ["]
#[doc = "    {"]
#[doc = "      \"description\": \"[Callgrind: a call-graph generating cache and branch prediction profiler](https://valgrind.org/docs/manual/cl-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Callgrind\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[Cachegrind: a high-precision tracing profiler](https://valgrind.org/docs/manual/cg-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Cachegrind\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[DHAT: a dynamic heap analysis tool](https://valgrind.org/docs/manual/dh-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"DHAT\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[Memcheck: a memory error detector](https://valgrind.org/docs/manual/mc-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Memcheck\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[Helgrind: a thread error detector](https://valgrind.org/docs/manual/hg-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Helgrind\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[DRD: a thread error detector](https://valgrind.org/docs/manual/drd-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"DRD\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"Massif\""]
#[doc = "    },"]
#[doc = "    {"]
#[doc = "      \"description\": \"[BBV: an experimental basic block vector generation tool](https://valgrind.org/docs/manual/bbv-manual.html)\","]
#[doc = "      \"type\": \"string\","]
#[doc = "      \"const\": \"BBV\""]
#[doc = "    }"]
#[doc = "  ]"]
#[doc = "}"]
#[doc = r" ```"]
#[doc = r" </details>"]
#[derive(:: serde :: Deserialize, :: serde :: Serialize, Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValgrindTool {
    #[doc = "[Callgrind: a call-graph generating cache and branch prediction profiler](https://valgrind.org/docs/manual/cl-manual.html)"]
    Callgrind,
    #[doc = "[Cachegrind: a high-precision tracing profiler](https://valgrind.org/docs/manual/cg-manual.html)"]
    Cachegrind,
    #[doc = "[DHAT: a dynamic heap analysis tool](https://valgrind.org/docs/manual/dh-manual.html)"]
    #[serde(rename = "DHAT")]
    Dhat,
    #[doc = "[Memcheck: a memory error detector](https://valgrind.org/docs/manual/mc-manual.html)"]
    Memcheck,
    #[doc = "[Helgrind: a thread error detector](https://valgrind.org/docs/manual/hg-manual.html)"]
    Helgrind,
    #[doc = "[DRD: a thread error detector](https://valgrind.org/docs/manual/drd-manual.html)"]
    #[serde(rename = "DRD")]
    Drd,
    #[doc = "[Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html)"]
    Massif,
    #[doc = "[BBV: an experimental basic block vector generation tool](https://valgrind.org/docs/manual/bbv-manual.html)"]
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
            Self::Cachegrind => write!(f, "Cachegrind"),
            Self::Dhat => write!(f, "DHAT"),
            Self::Memcheck => write!(f, "Memcheck"),
            Self::Helgrind => write!(f, "Helgrind"),
            Self::Drd => write!(f, "DRD"),
            Self::Massif => write!(f, "Massif"),
            Self::Bbv => write!(f, "BBV"),
        }
    }
}
impl ::std::str::FromStr for ValgrindTool {
    type Err = self::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, self::error::ConversionError> {
        match value {
            "Callgrind" => Ok(Self::Callgrind),
            "Cachegrind" => Ok(Self::Cachegrind),
            "DHAT" => Ok(Self::Dhat),
            "Memcheck" => Ok(Self::Memcheck),
            "Helgrind" => Ok(Self::Helgrind),
            "DRD" => Ok(Self::Drd),
            "Massif" => Ok(Self::Massif),
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
    pub struct BenchmarkSummary {
        baselines: ::std::result::Result<
            (
                ::std::option::Option<::std::string::String>,
                ::std::option::Option<::std::string::String>,
            ),
            ::std::string::String,
        >,
        benchmark_exe: ::std::result::Result<::std::string::String, ::std::string::String>,
        benchmark_file: ::std::result::Result<::std::string::String, ::std::string::String>,
        details: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        function_name: ::std::result::Result<::std::string::String, ::std::string::String>,
        id: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        kind: ::std::result::Result<super::BenchmarkKind, ::std::string::String>,
        module_path: ::std::result::Result<::std::string::String, ::std::string::String>,
        package_dir: ::std::result::Result<::std::string::String, ::std::string::String>,
        profiles: ::std::result::Result<super::Profiles, ::std::string::String>,
        project_root: ::std::result::Result<::std::string::String, ::std::string::String>,
        summary_output: ::std::result::Result<::std::option::Option<super::SummaryOutput>, ::std::string::String>,
        version: ::std::result::Result<::std::string::String, ::std::string::String>,
    }
    impl ::std::default::Default for BenchmarkSummary {
        fn default() -> Self {
            Self {
                baselines: Err("no value supplied for baselines".to_string()),
                benchmark_exe: Err("no value supplied for benchmark_exe".to_string()),
                benchmark_file: Err("no value supplied for benchmark_file".to_string()),
                details: Ok(Default::default()),
                function_name: Err("no value supplied for function_name".to_string()),
                id: Ok(Default::default()),
                kind: Err("no value supplied for kind".to_string()),
                module_path: Err("no value supplied for module_path".to_string()),
                package_dir: Err("no value supplied for package_dir".to_string()),
                profiles: Err("no value supplied for profiles".to_string()),
                project_root: Err("no value supplied for project_root".to_string()),
                summary_output: Ok(Default::default()),
                version: Err("no value supplied for version".to_string()),
            }
        }
    }
    impl BenchmarkSummary {
        pub fn baselines<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<(
                    ::std::option::Option<::std::string::String>,
                    ::std::option::Option<::std::string::String>,
                )>,
            T::Error: ::std::fmt::Display,
        {
            self.baselines = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for baselines: {}", e));
            self
        }
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
        pub fn profiles<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::Profiles>,
            T::Error: ::std::fmt::Display,
        {
            self.profiles = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for profiles: {}", e));
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
                baselines: value.baselines?,
                benchmark_exe: value.benchmark_exe?,
                benchmark_file: value.benchmark_file?,
                details: value.details?,
                function_name: value.function_name?,
                id: value.id?,
                kind: value.kind?,
                module_path: value.module_path?,
                package_dir: value.package_dir?,
                profiles: value.profiles?,
                project_root: value.project_root?,
                summary_output: value.summary_output?,
                version: value.version?,
            })
        }
    }
    impl ::std::convert::From<super::BenchmarkSummary> for BenchmarkSummary {
        fn from(value: super::BenchmarkSummary) -> Self {
            Self {
                baselines: Ok(value.baselines),
                benchmark_exe: Ok(value.benchmark_exe),
                benchmark_file: Ok(value.benchmark_file),
                details: Ok(value.details),
                function_name: Ok(value.function_name),
                id: Ok(value.id),
                kind: Ok(value.kind),
                module_path: Ok(value.module_path),
                package_dir: Ok(value.package_dir),
                profiles: Ok(value.profiles),
                project_root: Ok(value.project_root),
                summary_output: Ok(value.summary_output),
                version: Ok(value.version),
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
        metrics: ::std::result::Result<super::EitherOrBoth2, ::std::string::String>,
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
            T: ::std::convert::TryInto<super::EitherOrBoth2>,
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
    pub struct Profile {
        flamegraphs: ::std::result::Result<::std::vec::Vec<super::FlamegraphSummary>, ::std::string::String>,
        log_paths: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        out_paths: ::std::result::Result<::std::vec::Vec<::std::string::String>, ::std::string::String>,
        summaries: ::std::result::Result<super::ProfileData, ::std::string::String>,
        tool: ::std::result::Result<super::ValgrindTool, ::std::string::String>,
    }
    impl ::std::default::Default for Profile {
        fn default() -> Self {
            Self {
                flamegraphs: Err("no value supplied for flamegraphs".to_string()),
                log_paths: Err("no value supplied for log_paths".to_string()),
                out_paths: Err("no value supplied for out_paths".to_string()),
                summaries: Err("no value supplied for summaries".to_string()),
                tool: Err("no value supplied for tool".to_string()),
            }
        }
    }
    impl Profile {
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
        pub fn summaries<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ProfileData>,
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
    impl ::std::convert::TryFrom<Profile> for super::Profile {
        type Error = super::error::ConversionError;
        fn try_from(value: Profile) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                flamegraphs: value.flamegraphs?,
                log_paths: value.log_paths?,
                out_paths: value.out_paths?,
                summaries: value.summaries?,
                tool: value.tool?,
            })
        }
    }
    impl ::std::convert::From<super::Profile> for Profile {
        fn from(value: super::Profile) -> Self {
            Self {
                flamegraphs: Ok(value.flamegraphs),
                log_paths: Ok(value.log_paths),
                out_paths: Ok(value.out_paths),
                summaries: Ok(value.summaries),
                tool: Ok(value.tool),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ProfileData {
        parts: ::std::result::Result<::std::vec::Vec<super::ProfilePart>, ::std::string::String>,
        total: ::std::result::Result<super::ProfileTotal, ::std::string::String>,
    }
    impl ::std::default::Default for ProfileData {
        fn default() -> Self {
            Self {
                parts: Err("no value supplied for parts".to_string()),
                total: Err("no value supplied for total".to_string()),
            }
        }
    }
    impl ProfileData {
        pub fn parts<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::ProfilePart>>,
            T::Error: ::std::fmt::Display,
        {
            self.parts = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for parts: {}", e));
            self
        }
        pub fn total<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ProfileTotal>,
            T::Error: ::std::fmt::Display,
        {
            self.total = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for total: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<ProfileData> for super::ProfileData {
        type Error = super::error::ConversionError;
        fn try_from(value: ProfileData) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                parts: value.parts?,
                total: value.total?,
            })
        }
    }
    impl ::std::convert::From<super::ProfileData> for ProfileData {
        fn from(value: super::ProfileData) -> Self {
            Self {
                parts: Ok(value.parts),
                total: Ok(value.total),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ProfileInfo {
        command: ::std::result::Result<::std::string::String, ::std::string::String>,
        details: ::std::result::Result<::std::option::Option<::std::string::String>, ::std::string::String>,
        parent_pid: ::std::result::Result<::std::option::Option<i32>, ::std::string::String>,
        part: ::std::result::Result<::std::option::Option<u64>, ::std::string::String>,
        path: ::std::result::Result<::std::string::String, ::std::string::String>,
        pid: ::std::result::Result<i32, ::std::string::String>,
        thread: ::std::result::Result<::std::option::Option<u32>, ::std::string::String>,
    }
    impl ::std::default::Default for ProfileInfo {
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
    impl ProfileInfo {
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
    impl ::std::convert::TryFrom<ProfileInfo> for super::ProfileInfo {
        type Error = super::error::ConversionError;
        fn try_from(value: ProfileInfo) -> ::std::result::Result<Self, super::error::ConversionError> {
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
    impl ::std::convert::From<super::ProfileInfo> for ProfileInfo {
        fn from(value: super::ProfileInfo) -> Self {
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
    pub struct ProfilePart {
        details: ::std::result::Result<super::EitherOrBoth, ::std::string::String>,
        metrics_summary: ::std::result::Result<super::ToolMetricSummary, ::std::string::String>,
    }
    impl ::std::default::Default for ProfilePart {
        fn default() -> Self {
            Self {
                details: Err("no value supplied for details".to_string()),
                metrics_summary: Err("no value supplied for metrics_summary".to_string()),
            }
        }
    }
    impl ProfilePart {
        pub fn details<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::EitherOrBoth>,
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
    impl ::std::convert::TryFrom<ProfilePart> for super::ProfilePart {
        type Error = super::error::ConversionError;
        fn try_from(value: ProfilePart) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                details: value.details?,
                metrics_summary: value.metrics_summary?,
            })
        }
    }
    impl ::std::convert::From<super::ProfilePart> for ProfilePart {
        fn from(value: super::ProfilePart) -> Self {
            Self {
                details: Ok(value.details),
                metrics_summary: Ok(value.metrics_summary),
            }
        }
    }
    #[derive(Clone, Debug)]
    pub struct ProfileTotal {
        regressions: ::std::result::Result<::std::vec::Vec<super::ToolRegression>, ::std::string::String>,
        summary: ::std::result::Result<super::ToolMetricSummary, ::std::string::String>,
    }
    impl ::std::default::Default for ProfileTotal {
        fn default() -> Self {
            Self {
                regressions: Err("no value supplied for regressions".to_string()),
                summary: Err("no value supplied for summary".to_string()),
            }
        }
    }
    impl ProfileTotal {
        pub fn regressions<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<::std::vec::Vec<super::ToolRegression>>,
            T::Error: ::std::fmt::Display,
        {
            self.regressions = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for regressions: {}", e));
            self
        }
        pub fn summary<T>(mut self, value: T) -> Self
        where
            T: ::std::convert::TryInto<super::ToolMetricSummary>,
            T::Error: ::std::fmt::Display,
        {
            self.summary = value
                .try_into()
                .map_err(|e| format!("error converting supplied value for summary: {}", e));
            self
        }
    }
    impl ::std::convert::TryFrom<ProfileTotal> for super::ProfileTotal {
        type Error = super::error::ConversionError;
        fn try_from(value: ProfileTotal) -> ::std::result::Result<Self, super::error::ConversionError> {
            Ok(Self {
                regressions: value.regressions?,
                summary: value.summary?,
            })
        }
    }
    impl ::std::convert::From<super::ProfileTotal> for ProfileTotal {
        fn from(value: super::ProfileTotal) -> Self {
            Self {
                regressions: Ok(value.regressions),
                summary: Ok(value.summary),
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
}
