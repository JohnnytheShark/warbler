/**
 * A lightweight, zero-dependency Markdown to HTML parser.
 * Supports: Headers, Bold, Italic, Code Blocks, Inline Code, Lists, and Blockquotes.
 */
export function parseMarkdown(md: string): string {
    if (!md) return "";

    // Escape HTML special characters to prevent XSS
    let html = md
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;");

    // 1. Fenced Code Blocks (```lang ... ```)
    html = html.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
        const languageClass = lang ? `class="language-${lang}"` : "";
        return `<div class="code-block-container">
            ${lang ? `<div class="code-block-header">${lang}</div>` : ""}
            <pre><code ${languageClass}>${code.trim()}</code></pre>
        </div>`;
    });

    // 2. Inline Code (`code`)
    html = html.replace(/`([^`]+)`/g, "<code>$1</code>");

    // 3. Bold (**bold** or __bold__)
    html = html.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>");
    html = html.replace(/__(.*?)__/g, "<strong>$1</strong>");

    // 4. Italic (*italic* or _italic_)
    html = html.replace(/\*(.*?)\*/g, "<em>$1</em>");
    html = html.replace(/_(.*?)_/g, "<em>$1</em>");

    // 5. Blockquotes (> quote)
    html = html.replace(/^&gt; (.*$)/gm, "<blockquote>$1</blockquote>");

    // 6. Headers (# Header)
    html = html.replace(/^### (.*$)/gm, "<h3>$1</h3>");
    html = html.replace(/^## (.*$)/gm, "<h2>$1</h2>");
    html = html.replace(/^# (.*$)/gm, "<h1>$1</h1>");

    // 7. Unordered Lists (- item or * item)
    // We handle this by lines first, then wrapping contiguous <li>s in <ul>
    html = html.replace(/^\s*[-*] (.*$)/gm, "<li>$1</li>");
    
    // 8. Ordered Lists (1. item)
    html = html.replace(/^\s*\d+\. (.*$)/gm, "<li>$1</li>"); // Simplification: treat as same <li> style

    // Wrap consecutive <li> elements in <ul>
    // This is a simple lookahead/behind approximation
    html = html.replace(/(<li>[\s\S]*?<\/li>)/g, (match) => {
        return `<ul>${match}</ul>`;
    }).replace(/<\/ul>\s*<ul>/g, "");

    // 9. Paragraphs and Line Breaks
    // Split by double newlines to create paragraphs, but ignore blocks already wrapped in HTML tags
    const blocks = html.split(/\n\n+/);
    html = blocks.map(block => {
        const trimmed = block.trim();
        if (!trimmed) return "";
        
        // If it starts with a block-level tag, don't wrap in <p>
        const blockTags = /^(<h|<div|<pre|<blockquote|<ul|<li)/;
        if (blockTags.test(trimmed)) {
            return trimmed;
        }
        
        // Otherwise, wrap in <p> and convert single newlines to <br>
        return `<p>${trimmed.replace(/\n/g, "<br>")}</p>`;
    }).join("");

    return html;
}
