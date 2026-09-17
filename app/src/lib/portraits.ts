const files = import.meta.glob<string>("../assets/portraits/*.webp", {
  eager: true,
  query: "?url",
  import: "default",
});

const BY_KEY: Record<string, string> = {};
for (const [path, url] of Object.entries(files)) {
  BY_KEY[path.slice(path.lastIndexOf("/") + 1, -".webp".length)] = url;
}

export const charKey = (name: string) =>
  name.normalize("NFKC").replace(/[^\p{L}\p{N}]/gu, "").toLowerCase();

const FULL: Record<string, string> = {
  alisabosconovitch: "alisa",
  annawilliams: "anna",
  asukakazama: "asuka",
  bobrichards: "bob",
  bryanfury: "bryan",
  claudioserafino: "claudio",
  cliverosfield: "clive",
  sergeidragunov: "dragunov",
  eddygordo: "eddy",
  fengwei: "feng",
  heihachimishima: "heihachi",
  jinkazama: "jin",
  junkazama: "jun",
  kazuyamishima: "kazuya",
  larsalexandersson: "lars",
  marshalllaw: "law",
  leechaolan: "lee",
  leokliesen: "leo",
  leroysmith: "leroy",
  lidiasobieska: "lidia",
  lilirochefort: "lili",
  ninawilliams: "nina",
  paulphoenix: "paul",
  stevefox: "steve",
  victorchevalier: "victor",
  lingxiaoyu: "xiaoyu",
};

export function portraitFor(name: string | null | undefined): string | null {
  if (!name) return null;
  const key = charKey(name);
  return BY_KEY[key] ?? BY_KEY[FULL[key] ?? ""] ?? null;
}

export const PORTRAIT_KEYS = Object.keys(BY_KEY);
