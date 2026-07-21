use crate::CapabilityTag;
use serde::{Deserialize, Serialize};

/// Inferred compatibility profile for a provider model id.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelCompatProfile {
    pub display_name: String,
    pub capabilities: Vec<CapabilityTag>,
    pub context_length: Option<u32>,
    pub family: Option<&'static str>,
    /// When false, model is below the supported version floor for a known family.
    pub supported: bool,
}

/// Infer capabilities / display for a remote model id.
/// Version floors (recent ~1y only):
/// - GPT / OpenAI chat: 5.4+
/// - Claude: 4.5+
/// - MiniMax: M2.5+
/// - Qwen: 3.5+
/// - Kimi / Moonshot: K2.5+
/// Other families are kept and filled by name heuristics.
pub fn infer_model_compat(model_id: &str) -> ModelCompatProfile {
    let raw = model_id.trim();
    let id = raw.to_ascii_lowercase();
    let id = id.trim();

    if id.is_empty() {
        return ModelCompatProfile {
            display_name: raw.to_string(),
            capabilities: base_caps(),
            context_length: None,
            family: None,
            supported: true,
        };
    }

    // --- OpenAI / GPT ---
    if let Some(profile) = match_gpt(raw, id) {
        return profile;
    }
    // --- Claude ---
    if let Some(profile) = match_claude(raw, id) {
        return profile;
    }
    // --- MiniMax ---
    if let Some(profile) = match_minimax(raw, id) {
        return profile;
    }
    // --- Qwen ---
    if let Some(profile) = match_qwen(raw, id) {
        return profile;
    }
    // --- Kimi / Moonshot ---
    if let Some(profile) = match_kimi(raw, id) {
        return profile;
    }
    // --- Other common modern families (no hard floor) ---
    if let Some(profile) = match_other(raw, id) {
        return profile;
    }

    ModelCompatProfile {
        display_name: pretty_display(raw),
        capabilities: base_caps(),
        context_length: None,
        family: None,
        supported: true,
    }
}

/// Enrich a remote list: drop known outdated family versions, fill fields for the rest.
pub fn enrich_model_list(
    models: impl IntoIterator<Item = (String, Option<String>)>,
) -> Vec<crate::ModelInfo> {
    let mut out = Vec::new();
    for (model_name, display_name) in models {
        let profile = infer_model_compat(&model_name);
        if !profile.supported {
            continue;
        }
        let display = display_name
            .filter(|s| !s.trim().is_empty() && s != &model_name)
            .unwrap_or_else(|| profile.display_name.clone());
        out.push(crate::ModelInfo {
            model_name,
            display_name: display,
            capabilities: profile.capabilities,
            context_length: profile.context_length,
            family: profile.family.map(|s| s.to_string()),
        });
    }
    out.sort_by(|a, b| {
        a.display_name
            .to_ascii_lowercase()
            .cmp(&b.display_name.to_ascii_lowercase())
    });
    out
}

fn base_caps() -> Vec<CapabilityTag> {
    vec![CapabilityTag::Text, CapabilityTag::Streaming]
}

fn with_caps(extra: &[CapabilityTag]) -> Vec<CapabilityTag> {
    let mut caps = base_caps();
    for c in extra {
        if !caps.contains(c) {
            caps.push(*c);
        }
    }
    caps
}

fn pretty_display(raw: &str) -> String {
    raw.to_string()
}

fn match_gpt(raw: &str, id: &str) -> Option<ModelCompatProfile> {
    // Skip pure embedding / tts / whisper / dall-e / moderation utility models.
    if is_openai_utility(id) {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: vec![CapabilityTag::Text],
            context_length: None,
            family: Some("openai-utility"),
            supported: false,
        });
    }

    // gpt-5.x / chatgpt-5.x / gpt-5
    if let Some(ver) = extract_gpt_version(id) {
        let supported = ver.0 > 5 || (ver.0 == 5 && ver.1 >= 4);
        let mut extra = vec![
            CapabilityTag::Vision,
            CapabilityTag::ToolCalling,
            CapabilityTag::Reasoning,
        ];
        if id.contains("image") || id.contains("gpt-image") {
            extra.push(CapabilityTag::ImageGeneration);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: if supported {
                with_caps(&extra)
            } else {
                base_caps()
            },
            context_length: Some(if ver.0 >= 5 { 400_000 } else { 128_000 }),
            family: Some("gpt"),
            supported,
        });
    }

    // o-series (o3 / o4 / ...) treated as modern reasoning chat
    if let Some(n) = extract_o_series(id) {
        let supported = n >= 3;
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: if supported {
                with_caps(&[
                    CapabilityTag::Vision,
                    CapabilityTag::ToolCalling,
                    CapabilityTag::Reasoning,
                ])
            } else {
                base_caps()
            },
            context_length: Some(200_000),
            family: Some("openai-o"),
            supported,
        });
    }

    // legacy explicit gpt-4 / gpt-3.5
    if id.contains("gpt-4") || id.contains("gpt-3.5") || id.contains("gpt-3") {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: base_caps(),
            context_length: Some(128_000),
            family: Some("gpt"),
            supported: false,
        });
    }

    None
}

fn match_claude(raw: &str, id: &str) -> Option<ModelCompatProfile> {
    if !(id.contains("claude") || id.starts_with("anthropic.")) {
        return None;
    }
    let ver = extract_claude_version(id);
    let supported = match ver {
        Some((maj, min)) => maj > 4 || (maj == 4 && min >= 5),
        // undated "claude" without version — keep only if not clearly 3.x / 4.0
        None => {
            !(id.contains("claude-3")
                || id.contains("claude-2")
                || id.contains("claude-instant")
                || id.contains("claude-4-0")
                || id.contains("claude-4.0")
                || id.contains("claude-sonnet-4-0")
                || id.contains("claude-opus-4-0")
                || id.contains("claude-haiku-4-0")
                || id.contains("claude-sonnet-4@")
                || (id.contains("claude-4")
                    && !id.contains("4-5")
                    && !id.contains("4.5")
                    && !id.contains("4-6")
                    && !id.contains("4.6")))
        }
    };
    // If we detected version below floor
    if let Some((maj, min)) = ver {
        if !(maj > 4 || (maj == 4 && min >= 5)) {
            return Some(ModelCompatProfile {
                display_name: pretty_display(raw),
                capabilities: base_caps(),
                context_length: Some(200_000),
                family: Some("claude"),
                supported: false,
            });
        }
    } else if !supported {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: base_caps(),
            context_length: Some(200_000),
            family: Some("claude"),
            supported: false,
        });
    }

    Some(ModelCompatProfile {
        display_name: pretty_display(raw),
        capabilities: with_caps(&[
            CapabilityTag::Vision,
            CapabilityTag::ToolCalling,
            CapabilityTag::Reasoning,
        ]),
        context_length: Some(200_000),
        family: Some("claude"),
        supported: true,
    })
}

fn match_minimax(raw: &str, id: &str) -> Option<ModelCompatProfile> {
    if !(id.contains("minimax")
        || id.contains("abab")
        || id.contains("m2.5")
        || id.contains("m2-5")
        || id.starts_with("m2.")
        || id.contains("-m2.")
        || id.contains("_m2."))
    {
        // MiniMax chat often: MiniMax-M2.5, minimax-text-01 — require hint
        if !id.contains("minimax") && !id.contains("abab") {
            return None;
        }
    }
    if !id.contains("minimax") && !id.contains("abab") && !id.contains("m2") {
        return None;
    }
    // Focus on MiniMax M series
    if id.contains("minimax")
        || id.contains("abab")
        || id.contains("-m2")
        || id.contains("_m2")
        || id.starts_with("m2")
    {
        let ver = extract_m_version(id).or_else(|| {
            if id.contains("m2.5") || id.contains("m2-5") {
                Some((2, 5))
            } else if id.contains("m2.1") || id.contains("m2-1") {
                Some((2, 1))
            } else if id.contains("m2") {
                Some((2, 0))
            } else {
                None
            }
        });
        if let Some((maj, min)) = ver {
            let supported = maj > 2 || (maj == 2 && min >= 5);
            return Some(ModelCompatProfile {
                display_name: pretty_display(raw),
                capabilities: if supported {
                    with_caps(&[
                        CapabilityTag::Vision,
                        CapabilityTag::ToolCalling,
                        CapabilityTag::Reasoning,
                    ])
                } else {
                    base_caps()
                },
                context_length: Some(1_000_000),
                family: Some("minimax"),
                supported,
            });
        }
        // named minimax without m-version: keep if not clearly abab6 / old
        if id.contains("abab") && !id.contains("m2") {
            return Some(ModelCompatProfile {
                display_name: pretty_display(raw),
                capabilities: base_caps(),
                context_length: None,
                family: Some("minimax"),
                supported: false,
            });
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&[CapabilityTag::ToolCalling, CapabilityTag::Vision]),
            context_length: Some(1_000_000),
            family: Some("minimax"),
            supported: true,
        });
    }
    None
}

fn match_qwen(raw: &str, id: &str) -> Option<ModelCompatProfile> {
    if !(id.contains("qwen") || id.contains("qwq") || id.contains("qvq")) {
        return None;
    }
    let ver = extract_qwen_version(id);
    if let Some((maj, min)) = ver {
        let supported = maj > 3 || (maj == 3 && min >= 5);
        if !supported {
            return Some(ModelCompatProfile {
                display_name: pretty_display(raw),
                capabilities: base_caps(),
                context_length: Some(128_000),
                family: Some("qwen"),
                supported: false,
            });
        }
        let mut extra = vec![CapabilityTag::ToolCalling, CapabilityTag::Reasoning];
        if id.contains("vl") || id.contains("vision") || id.contains("omni") {
            extra.push(CapabilityTag::Vision);
        }
        if id.contains("image") || id.contains("t2i") {
            extra.push(CapabilityTag::ImageGeneration);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(262_144),
            family: Some("qwen"),
            supported: true,
        });
    }
    // qwen-max / qwen-plus / qwen-turbo without number: treat as current commercial line
    if id.contains("qwen-max")
        || id.contains("qwen-plus")
        || id.contains("qwen-turbo")
        || id.contains("qwen-long")
        || id.contains("qwq")
        || id.contains("qvq")
    {
        let mut extra = vec![CapabilityTag::ToolCalling, CapabilityTag::Reasoning];
        if id.contains("vl") || id.contains("vision") || id.contains("qvq") {
            extra.push(CapabilityTag::Vision);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(262_144),
            family: Some("qwen"),
            supported: true,
        });
    }
    // qwen2 / qwen2.5 / qwen3 without .5 — floors
    if id.contains("qwen2") || id.contains("qwen-2") || id.contains("qwen1") {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: base_caps(),
            context_length: Some(128_000),
            family: Some("qwen"),
            supported: false,
        });
    }
    if id.contains("qwen3") && !id.contains("3.5") && !id.contains("3-5") {
        // qwen3 base line below 3.5 floor
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: base_caps(),
            context_length: Some(128_000),
            family: Some("qwen"),
            supported: false,
        });
    }
    Some(ModelCompatProfile {
        display_name: pretty_display(raw),
        capabilities: with_caps(&[CapabilityTag::ToolCalling]),
        context_length: None,
        family: Some("qwen"),
        supported: true,
    })
}

fn match_kimi(raw: &str, id: &str) -> Option<ModelCompatProfile> {
    let is_kimi = id.contains("kimi")
        || id.contains("moonshot")
        || id.contains("k2.5")
        || id.contains("k2-5")
        || id.contains("kimi-k2");
    if !is_kimi {
        return None;
    }
    // moonshot-v1 is old
    if id.contains("moonshot-v1") || id.contains("moonshot-v0") {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: base_caps(),
            context_length: Some(128_000),
            family: Some("kimi"),
            supported: false,
        });
    }
    let ver = extract_k_version(id);
    if let Some((maj, min)) = ver {
        let supported = maj > 2 || (maj == 2 && min >= 5);
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: if supported {
                with_caps(&[
                    CapabilityTag::Vision,
                    CapabilityTag::ToolCalling,
                    CapabilityTag::Reasoning,
                ])
            } else {
                base_caps()
            },
            context_length: Some(262_144),
            family: Some("kimi"),
            supported,
        });
    }
    // kimi / moonshot without version: keep current commercial names
    if id.contains("kimi") || id.contains("moonshot") {
        // exclude clearly old k1 / k2.0 / k2.1
        if id.contains("k1")
            || id.contains("k2.0")
            || id.contains("k2-0")
            || id.contains("k2.1")
            || id.contains("k2-1")
        {
            return Some(ModelCompatProfile {
                display_name: pretty_display(raw),
                capabilities: base_caps(),
                context_length: Some(128_000),
                family: Some("kimi"),
                supported: false,
            });
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&[
                CapabilityTag::Vision,
                CapabilityTag::ToolCalling,
                CapabilityTag::Reasoning,
            ]),
            context_length: Some(262_144),
            family: Some("kimi"),
            supported: true,
        });
    }
    None
}

fn match_other(raw: &str, id: &str) -> Option<ModelCompatProfile> {
    // DeepSeek
    if id.contains("deepseek") {
        let mut extra = vec![CapabilityTag::ToolCalling, CapabilityTag::Reasoning];
        if id.contains("vl") || id.contains("vision") {
            extra.push(CapabilityTag::Vision);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(128_000),
            family: Some("deepseek"),
            supported: true,
        });
    }
    // Gemini
    if id.contains("gemini") {
        let mut extra = vec![
            CapabilityTag::Vision,
            CapabilityTag::ToolCalling,
            CapabilityTag::Reasoning,
        ];
        if id.contains("image") {
            extra.push(CapabilityTag::ImageGeneration);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(1_000_000),
            family: Some("gemini"),
            supported: true,
        });
    }
    // Grok
    if id.contains("grok") {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&[
                CapabilityTag::Vision,
                CapabilityTag::ToolCalling,
                CapabilityTag::Reasoning,
            ]),
            context_length: Some(256_000),
            family: Some("grok"),
            supported: true,
        });
    }
    // Mistral / Mixtral
    if id.contains("mistral")
        || id.contains("mixtral")
        || id.contains("codestral")
        || id.contains("pixtral")
    {
        let mut extra = vec![CapabilityTag::ToolCalling];
        if id.contains("pixtral") || id.contains("vision") {
            extra.push(CapabilityTag::Vision);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(128_000),
            family: Some("mistral"),
            supported: true,
        });
    }
    // Llama
    if id.contains("llama") {
        let mut extra = vec![CapabilityTag::ToolCalling];
        if id.contains("vision") || id.contains("llava") || id.contains("11b-vision") {
            extra.push(CapabilityTag::Vision);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(128_000),
            family: Some("llama"),
            supported: true,
        });
    }
    // GLM
    if id.contains("glm") {
        let mut extra = vec![CapabilityTag::ToolCalling, CapabilityTag::Reasoning];
        if id.contains("v") && (id.contains("4v") || id.contains("vl") || id.contains("vision")) {
            extra.push(CapabilityTag::Vision);
        }
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&extra),
            context_length: Some(128_000),
            family: Some("glm"),
            supported: true,
        });
    }
    // Doubao
    if id.contains("doubao") || id.contains("ep-") {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: with_caps(&[CapabilityTag::ToolCalling, CapabilityTag::Vision]),
            context_length: Some(256_000),
            family: Some("doubao"),
            supported: true,
        });
    }
    // generic image models
    if id.contains("dall-e")
        || id.contains("flux")
        || id.contains("stable-diffusion")
        || id.contains("sdxl")
    {
        return Some(ModelCompatProfile {
            display_name: pretty_display(raw),
            capabilities: vec![CapabilityTag::ImageGeneration],
            context_length: None,
            family: Some("image"),
            supported: true,
        });
    }
    None
}

fn is_openai_utility(id: &str) -> bool {
    id.contains("embedding")
        || id.contains("whisper")
        || id.contains("tts")
        || id.contains("davinci")
        || id.contains("babbage")
        || id.contains("moderation")
        || id.contains("realtime")
        || id.starts_with("ft:")
}

/// gpt-5.4 / gpt-5.4-mini / chatgpt-4o-latest → Some((5,4))
fn extract_gpt_version(id: &str) -> Option<(u32, u32)> {
    // find "gpt-" then version
    let markers = ["gpt-", "chatgpt-"];
    for m in markers {
        if let Some(pos) = id.find(m) {
            let rest = &id[pos + m.len()..];
            if let Some(v) = parse_major_minor_prefix(rest) {
                return Some(v);
            }
        }
    }
    None
}

fn extract_o_series(id: &str) -> Option<u32> {
    // o3, o3-mini, o4-mini, openai/o3
    let re_pos = id.find("o");
    // scan for pattern \bo(\d+)
    let bytes = id.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let is_boundary = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
        if is_boundary && bytes[i] == b'o' && bytes[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j == i + 1 {
                i += 1;
                continue;
            }
            // next should be end, -, or .
            if j < bytes.len() {
                let c = bytes[j] as char;
                if c.is_ascii_alphanumeric() && c != '-' {
                    i += 1;
                    continue;
                }
            }
            let n: u32 = id[i + 1..j].parse().ok()?;
            return Some(n);
        }
        i += 1;
    }
    let _ = re_pos;
    None
}

fn extract_claude_version(id: &str) -> Option<(u32, u32)> {
    // claude-sonnet-4-5-20250929, claude-4.5, claude-opus-4-5, anthropic.claude-sonnet-4-5
    // Prefer 4-5 / 4.5 patterns after claude
    let patterns_dash = [
        "-4-5", "-4.5", "-4-6", "-4.6", "-5-0", "-5.0", "-3-5", "-3.5", "-3-7", "-3.7", "-4-0",
        "-4.0", "-4-1", "-4.1",
    ];
    for p in patterns_dash {
        if id.contains(p) {
            let digits: String = p
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            // e.g. -4-5 → 4,5
            let cleaned = digits.trim_matches('-');
            if let Some(v) = parse_major_minor_flexible(cleaned) {
                return Some(v);
            }
        }
    }
    // claude-4-5 anywhere
    if let Some(pos) = id.find("claude") {
        let rest = &id[pos..];
        // find first digit sequence like 4.5 or 4-5
        return find_version_in(rest);
    }
    None
}

fn extract_qwen_version(id: &str) -> Option<(u32, u32)> {
    // Prefer dotted forms: qwen3.5 / qwen-3.5 / qwen2.5
    // "qwen3-32b" is parameter size, NOT minor 32.
    for marker in ["qwen", "qwq", "qvq"] {
        if let Some(pos) = id.find(marker) {
            let rest = &id[pos + marker.len()..];
            let rest = rest.trim_start_matches(|c: char| c == '-' || c == '_' || c == ' ');
            if let Some(v) = parse_major_minor_dotted(rest) {
                return Some(v);
            }
            // bare major only: qwen3-32b / qwen3
            if let Some(major) = parse_major_only(rest) {
                return Some((major, 0));
            }
        }
    }
    None
}

/// major.minor only when minor uses '.'; hyphen digits after major are sizes (32b).
fn parse_major_minor_dotted(s: &str) -> Option<(u32, u32)> {
    let s = s.trim_start_matches(|c: char| !c.is_ascii_digit());
    if s.is_empty() || !s.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let b = s.as_bytes();
    let mut i = 0;
    let mut major = 0u32;
    while i < b.len() && b[i].is_ascii_digit() {
        major = major
            .saturating_mul(10)
            .saturating_add((b[i] - b'0') as u32);
        i += 1;
    }
    if i == 0 || i >= b.len() || b[i] != b'.' {
        return None;
    }
    i += 1;
    if i >= b.len() || !b[i].is_ascii_digit() {
        return None;
    }
    let mut minor = 0u32;
    while i < b.len() && b[i].is_ascii_digit() {
        minor = minor
            .saturating_mul(10)
            .saturating_add((b[i] - b'0') as u32);
        i += 1;
    }
    Some((major, minor))
}

fn parse_major_only(s: &str) -> Option<u32> {
    let s = s.trim_start_matches(|c: char| !c.is_ascii_digit());
    if s.is_empty() || !s.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let b = s.as_bytes();
    let mut i = 0;
    let mut major = 0u32;
    while i < b.len() && b[i].is_ascii_digit() {
        major = major
            .saturating_mul(10)
            .saturating_add((b[i] - b'0') as u32);
        i += 1;
    }
    // reject if immediately continues as dotted (handled elsewhere)
    if i < b.len() && b[i] == b'.' {
        return None;
    }
    Some(major)
}

fn extract_m_version(id: &str) -> Option<(u32, u32)> {
    // MiniMax-M2.5 / minimax-m2.5 / m2.5
    for marker in ["minimax-m", "minimax_m", "-m", "_m", "m"] {
        if let Some(pos) = id.rfind(marker) {
            let rest = &id[pos + marker.len()..];
            if let Some(v) = parse_major_minor_prefix(rest) {
                // avoid matching random m in middle of words: require digit start
                return Some(v);
            }
        }
    }
    find_version_after_letter(id, 'm')
}

fn extract_k_version(id: &str) -> Option<(u32, u32)> {
    // kimi-k2.5 / k2.5 / moonshot-k2.5
    for marker in ["kimi-k", "kimi_k", "-k", "_k"] {
        if let Some(pos) = id.find(marker) {
            let rest = &id[pos + marker.len()..];
            if let Some(v) = parse_major_minor_prefix(rest) {
                return Some(v);
            }
        }
    }
    // bare k2.5
    if let Some(pos) = id.find("k2") {
        let rest = &id[pos + 1..];
        if let Some(v) = parse_major_minor_prefix(rest) {
            return Some(v);
        }
    }
    None
}

fn find_version_in(s: &str) -> Option<(u32, u32)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            if let Some(v) = parse_major_minor_prefix(&s[i..]) {
                return Some(v);
            }
        }
        i += 1;
    }
    None
}

fn find_version_after_letter(s: &str, letter: char) -> Option<(u32, u32)> {
    let l = letter.to_ascii_lowercase();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if (bytes[i] as char).to_ascii_lowercase() == l && bytes[i + 1].is_ascii_digit() {
            let boundary = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            if boundary {
                if let Some(v) = parse_major_minor_prefix(&s[i + 1..]) {
                    return Some(v);
                }
            }
        }
        i += 1;
    }
    None
}

fn parse_major_minor_prefix(s: &str) -> Option<(u32, u32)> {
    let s = s.trim_start_matches(|c: char| !c.is_ascii_digit());
    if s.is_empty() || !s.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let mut major = 0u32;
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len() && b[i].is_ascii_digit() {
        major = major
            .saturating_mul(10)
            .saturating_add((b[i] - b'0') as u32);
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let mut minor = 0u32;
    if i < b.len() && (b[i] == b'.' || b[i] == b'-') {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            minor = minor
                .saturating_mul(10)
                .saturating_add((b[i] - b'0') as u32);
            i += 1;
        }
        if i == start {
            minor = 0;
        }
    }
    Some((major, minor))
}

fn parse_major_minor_flexible(s: &str) -> Option<(u32, u32)> {
    let s = s.replace('-', ".");
    parse_major_minor_prefix(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpt_floor_5_4() {
        assert!(infer_model_compat("gpt-5.4").supported);
        assert!(infer_model_compat("gpt-5.4-mini").supported);
        assert!(infer_model_compat("gpt-5.5").supported);
        assert!(!infer_model_compat("gpt-5.3").supported);
        assert!(!infer_model_compat("gpt-5").supported);
        assert!(!infer_model_compat("gpt-4o").supported);
        assert!(!infer_model_compat("gpt-4.1").supported);
        assert!(infer_model_compat("gpt-5.4")
            .capabilities
            .contains(&CapabilityTag::Vision));
        assert!(infer_model_compat("gpt-5.4")
            .capabilities
            .contains(&CapabilityTag::Reasoning));
    }

    #[test]
    fn claude_floor_4_5() {
        assert!(infer_model_compat("claude-sonnet-4-5-20250929").supported);
        assert!(infer_model_compat("claude-opus-4.5").supported);
        assert!(!infer_model_compat("claude-3-5-sonnet-20241022").supported);
        assert!(!infer_model_compat("claude-sonnet-4-0").supported);
        assert!(infer_model_compat("claude-sonnet-4-5-20250929")
            .capabilities
            .contains(&CapabilityTag::ToolCalling));
    }

    #[test]
    fn minimax_floor_m2_5() {
        assert!(infer_model_compat("MiniMax-M2.5").supported);
        assert!(infer_model_compat("minimax-m2.5").supported);
        assert!(!infer_model_compat("MiniMax-M2.1").supported);
        assert!(!infer_model_compat("abab6.5s-chat").supported);
    }

    #[test]
    fn qwen_floor_3_5() {
        assert!(infer_model_compat("qwen3.5-plus").supported);
        assert!(infer_model_compat("qwen-3.5").supported);
        assert!(!infer_model_compat("qwen2.5-72b-instruct").supported);
        assert!(!infer_model_compat("qwen3-32b").supported);
        assert!(infer_model_compat("qwen3.5-vl")
            .capabilities
            .contains(&CapabilityTag::Vision));
    }

    #[test]
    fn kimi_floor_k2_5() {
        assert!(infer_model_compat("kimi-k2.5").supported);
        assert!(infer_model_compat("moonshot-k2.5").supported);
        assert!(!infer_model_compat("moonshot-v1-128k").supported);
        assert!(!infer_model_compat("kimi-k2.1").supported);
    }

    #[test]
    fn enrich_filters_old() {
        let list = enrich_model_list([
            ("gpt-4o".into(), None),
            ("gpt-5.4".into(), None),
            ("claude-3-5-sonnet".into(), None),
            ("claude-sonnet-4-5".into(), Some("Claude Sonnet 4.5".into())),
        ]);
        let names: Vec<_> = list.iter().map(|m| m.model_name.as_str()).collect();
        assert!(names.contains(&"gpt-5.4"));
        assert!(names.contains(&"claude-sonnet-4-5"));
        assert!(!names.contains(&"gpt-4o"));
        assert!(!names.contains(&"claude-3-5-sonnet"));
    }

    #[test]
    fn o_series() {
        assert!(infer_model_compat("o3").supported);
        assert!(infer_model_compat("o4-mini").supported);
        assert!(!infer_model_compat("o1-preview").supported);
    }
}
