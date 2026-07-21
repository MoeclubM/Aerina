use aerina_domain::*;
use aerina_providers::build_provider;
use futures::StreamExt;
use std::env;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .local/live-test.env if present (KEY=VALUE lines).
    if let Ok(text) = std::fs::read_to_string(".local/live-test.env") {
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                // Do not override already-exported env.
                if env::var(k).is_err() {
                    // SAFETY: process-local env mutation before concurrent work.
                    unsafe { env::set_var(k, v) };
                }
            }
        }
    }

    let base_url = env::var("AERINA_LIVE_BASE_URL")
        .expect("AERINA_LIVE_BASE_URL required (see .local/live-test.env)");
    let api_key = env::var("AERINA_LIVE_API_KEY")
        .expect("AERINA_LIVE_API_KEY required (see .local/live-test.env)");
    let model = env::var("AERINA_LIVE_MODEL").unwrap_or_else(|_| "grok-4.5".into());
    let kind =
        env::var("AERINA_LIVE_PROVIDER_KIND").unwrap_or_else(|_| "open_ai_compatible".into());
    let provider_kind = match kind.as_str() {
        "open_ai_responses" => ProviderKind::OpenAiResponses,
        "anthropic" => ProviderKind::Anthropic,
        "open_ai" => ProviderKind::OpenAi,
        _ => ProviderKind::OpenAiCompatible,
    };

    let provider = build_provider(ProviderConfig {
        id: ProviderId::new(),
        name: "live-test".into(),
        kind: provider_kind,
        base_url,
        api_key: Some(api_key),
    })?;

    let request = TextGenerationRequest {
        model,
        messages: vec![ChatMessage::text("user", "Reply with exactly: pong")],
        temperature: Some(0.0),
        system_prompt: Some("Be extremely concise.".into()),
        tools: Vec::new(),
        tool_choice: None,
    };

    let mut stream = provider
        .generate_stream("live", "A", request, CancellationToken::new())
        .await?;

    let mut text = String::new();
    while let Some(event) = stream.next().await {
        match event {
            GenerationEvent::TextDelta { delta, .. } => {
                print!("{delta}");
                text.push_str(&delta);
            }
            GenerationEvent::Error { message, .. } => {
                eprintln!("\nERROR: {message}");
                std::process::exit(1);
            }
            GenerationEvent::Done { .. } => break,
            GenerationEvent::Usage { usage, .. } => {
                eprintln!(
                    "\n[usage tokens={:?} latency_ms={:?}]",
                    usage.total_tokens, usage.latency_ms
                );
            }
            _ => {}
        }
    }
    println!();
    if text.trim().is_empty() {
        anyhow::bail!("empty response from live endpoint");
    }
    println!(
        "OK live chat ({provider_kind:?}) chars={}",
        text.chars().count()
    );
    Ok(())
}
