import type { CSSProperties } from "react";

import rarity1 from "@/assets/rarity-1.svg";
import rarity2 from "@/assets/rarity-2.svg";
import rarity3 from "@/assets/rarity-3.svg";
import rarity4 from "@/assets/rarity-4.svg";
import rarity5 from "@/assets/rarity-5.svg";

export type Rarity = 1 | 2 | 3 | 4 | 5;

export const RARITY_ICON: Record<number, string> = { 1: rarity1, 2: rarity2, 3: rarity3, 4: rarity4, 5: rarity5 };

export const RARITY_NAME: Record<number, string> = {
  1: "Common",
  2: "Uncommon",
  3: "Rare",
  4: "Epic",
  5: "Legendary",
};

export const rarityVar = (n: number): CSSProperties =>
  ({ "--rarity": `var(--rarity-${Math.min(5, Math.max(1, n))})` }) as CSSProperties;
