// Markdown files are bundled as text by the Wrangler "Text" rule.
declare module "*.md" {
  const text: string;
  export default text;
}
