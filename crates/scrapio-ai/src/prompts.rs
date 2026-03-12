//! Prompt templates for AI extraction

pub fn extraction_system_prompt() -> &'static str {
    r#"You are an expert web scraper. Your task is to extract structured data from web page content.
        Analyze the HTML/text content and extract meaningful information based on the user's schema.
        Return valid JSON that matches the schema exactly."#
}

pub fn extraction_user_prompt(content: &str, schema: &str) -> String {
    format!(
        "Extract data from the following content using this JSON schema:\n\nSchema:\n{}\n\nContent:\n{}\n\nReturn ONLY valid JSON.",
        schema, content
    )
}
