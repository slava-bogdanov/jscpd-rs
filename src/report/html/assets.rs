pub(super) const TAILWIND_CSS: &str = r#"
body { margin: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #f3f4f6; color: #1f2937; }
.container { max-width: 1200px; margin: 0 auto; padding-left: 1rem; padding-right: 1rem; }
header { background: #fff; box-shadow: 0 1px 3px rgb(0 0 0 / 0.12); padding: 1rem 0; }
main { background: #fff; margin-top: 2rem; margin-bottom: 2rem; padding: 1rem; box-shadow: 0 1px 3px rgb(0 0 0 / 0.12); border-radius: 0.25rem; }
h1 { margin: 0; font-size: 1.875rem; line-height: 2.25rem; font-weight: 600; }
h2 { margin: 0 0 1rem; font-size: 1.5rem; line-height: 2rem; font-weight: 600; color: #374151; }
h3 { margin: 0 0 0.5rem; font-size: 1.125rem; line-height: 1.75rem; font-weight: 600; }
section { margin-bottom: 2rem; }
table { width: 100%; border-collapse: collapse; }
th { background: #e5e7eb; color: #4b5563; font-size: 0.875rem; line-height: 1.25rem; padding: 0.75rem 1.5rem; text-align: left; text-transform: uppercase; }
td { border-bottom: 1px solid #e5e7eb; color: #1f2937; font-size: 0.875rem; line-height: 1.25rem; padding: 0.75rem 1.5rem; }
a { color: #2563eb; text-decoration: none; }
a:hover { text-decoration: underline; }
button { background: #6b7280; border: 0; border-radius: 0.25rem; color: #fff; cursor: pointer; font-size: 0.75rem; line-height: 1rem; margin-left: 0.5rem; padding: 0.125rem 0.25rem; }
pre { background: #f3f4f6; border: 1px solid #e5e7eb; border-radius: 0.25rem; margin-top: 0.5rem; overflow: auto; padding: 1rem; }
code { font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace; font-size: 0.875rem; line-height: 1.25rem; }
.dashboard-grid { display: grid; gap: 1rem; grid-template-columns: repeat(4, minmax(0, 1fr)); }
.card { border-radius: 0.25rem; padding: 1rem; text-align: center; }
.card span { display: block; font-size: 2.25rem; line-height: 2.5rem; font-weight: 700; }
.blue { background: #bfdbfe; color: #1e40af; }
.green { background: #bbf7d0; color: #166534; }
.yellow { background: #fef08a; color: #854d0e; }
.red { background: #fecaca; color: #991b1b; }
.clone { border-top: 1px solid #e5e7eb; padding: 1rem 0; }
.clone:first-child { border-top: 0; }
.clone p { margin: 0 0 0.5rem; color: #4b5563; }
.hidden { display: none; }
footer { margin-top: 60px; padding: 30px 0; border-top: 1px solid #e0e0e0; text-align: center; color: #666; }
@media (max-width: 800px) {
  .dashboard-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  th, td { padding: 0.5rem; }
}
@media (max-width: 520px) {
  .dashboard-grid { grid-template-columns: 1fr; }
  main { margin-top: 1rem; }
}
"#;

pub(super) const PRISM_CSS: &str = r#"
pre[class*="language-"] { white-space: pre-wrap; word-break: break-word; }
code[class*="language-"] { color: #111827; }
"#;

pub(super) const PRISM_JS: &str = r#"
window.Prism = window.Prism || { highlightAll: function () {} };
"#;
