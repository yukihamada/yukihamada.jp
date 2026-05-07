use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct BlogPost {
    pub slug: String,
    pub title: String,
    pub date: String,
    pub description: String,
    pub tags: Vec<String>,
    pub html: String,
    pub word_count: usize,
    pub reading_minutes: usize,
}

fn parse_frontmatter(content: &str) -> (BlogPost, &str) {
    let content = content.trim_start();
    let mut post = BlogPost {
        slug: String::new(),
        title: String::new(),
        date: String::new(),
        description: String::new(),
        tags: Vec::new(),
        html: String::new(),
        word_count: 0,
        reading_minutes: 1,
    };

    if !content.starts_with("---") {
        return (post, content);
    }

    let after_first = &content[3..];
    let end = after_first.find("---").unwrap_or(0);
    let frontmatter = &after_first[..end];
    let body = &after_first[end + 3..];

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("title:") {
            post.title = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = line.strip_prefix("date:") {
            post.date = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = line.strip_prefix("description:") {
            post.description = val.trim().trim_matches('"').to_string();
        } else if let Some(val) = line.strip_prefix("tags:") {
            let val = val.trim().trim_start_matches('[').trim_end_matches(']');
            post.tags = val
                .split(',')
                .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|t| !t.is_empty())
                .collect();
        }
    }

    (post, body)
}

fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let parser = Parser::new_ext(md, opts);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn count_reading_time(body: &str) -> (usize, usize) {
    let words: usize = body.split_whitespace().count();
    let cjk: usize = body.chars().filter(|c| *c >= '\u{3000}' && *c <= '\u{9fff}').count();
    let total = words + cjk;
    let minutes = (total / 400).max(1);
    (total, minutes)
}

pub fn load_posts(dir: &Path) -> Vec<BlogPost> {
    let mut posts = Vec::new();

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return posts,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let filename = path.file_stem().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(&path).unwrap_or_default();
        let (mut post, body) = parse_frontmatter(&content);
        post.slug = filename;
        post.html = render_markdown(body);

        let (wc, rm) = count_reading_time(body);
        post.word_count = wc;
        post.reading_minutes = rm;

        if post.title.is_empty() {
            for line in body.lines() {
                if let Some(h) = line.strip_prefix("# ") {
                    post.title = h.to_string();
                    break;
                }
            }
        }

        posts.push(post);
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    posts
}

/// Get unique tags across all posts, sorted by frequency
pub fn collect_tags(posts: &[BlogPost]) -> Vec<(String, usize)> {
    let mut map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for post in posts {
        for tag in &post.tags {
            *map.entry(tag.clone()).or_insert(0) += 1;
        }
    }
    let mut tags: Vec<_> = map.into_iter().collect();
    tags.sort_by(|a, b| b.1.cmp(&a.1));
    tags
}
