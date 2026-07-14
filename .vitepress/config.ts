import { readFileSync } from "node:fs";
import { defineConfig, type DefaultTheme } from "vitepress";

type WikiMeta = {
  slug: string;
  title: string;
  description: string;
  lang?: string;
  github?: string;
};

function loadWikiMeta(): WikiMeta {
  const raw = readFileSync(".wiki.json", "utf8");
  return JSON.parse(raw) as WikiMeta;
}

const wiki = loadWikiMeta();
const base = `/${wiki.slug}/wiki/`;

const socialLinks: DefaultTheme.SocialLink[] = wiki.github
  ? [{ icon: "github", link: wiki.github }]
  : [];

export default defineConfig({
  title: wiki.title,
  description: wiki.description,
  lang: wiki.lang ?? "ru-RU",
  base,
  srcDir: "content",
  cleanUrls: true,
  lastUpdated: true,
  head: [["meta", { name: "robots", content: "noindex, nofollow, noarchive" }]],
  themeConfig: {
    nav: [{ text: "Home", link: "/" }],
    sidebar: [
      {
        text: "Design",
        items: [
          { text: "Overview", link: "/" },
          { text: "Сводный дизайн", link: "/design" },
          { text: "К реализации", link: "/implementation" },
          { text: "UX", link: "/ux" },
          { text: "Dev (Rust)", link: "/dev" },
        ],
      },
    ],
    socialLinks,
    search: {
      provider: "local",
    },
    lastUpdated: {
      text: "Обновлено",
      formatOptions: {
        dateStyle: "medium",
        timeStyle: "short",
      },
    },
  },
});
