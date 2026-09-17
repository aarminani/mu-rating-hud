# Tekken Resource Hub Mu Rating HUD (True Prowess)

In Tekken 8, the ranked system has been a major point of contention for some time now, especially
when Street Fighter, one of our biggest competitors in the space has a very practical system that
effectively places players of equal skill in front of each other. This creates a more rewarding
environment and has likely kept both high level players and intermediate players interested
in Street Fighter 6 despite its many drawbacks.

## 📝 What is a Mu Rating?

Street Fighter uses Master Rate & I didn't want to directly take from them. I decided to go with
Mu Rating, since it's the foundational aspect of what traditional Mean Skill Estimation comes from.
It's also perfect, because Wavu Wank uses Mu, or μ, to list their scores.

## 📊 How It Works

Wavu Wank uses its own version of Glicko-2, and it's reinterpreted into this mod. Mu is determined by your
individual character performance in ranked matches. Unlike Tekken Prowess, which inflates based on
how many side characters you've leveled up.. it calculates an independent rating for each character by mapping
your match outcomes against your opponent's skill level. The mod uses a predictive, Shadow Glicko-2 model
to update your rating instantly after a fight, before continuously syncing with the global Wavu Wank API.

**μ** is the rating itself: Wavu's estimate of how strong that account is *on that character*. It
is per character, not per player, which is why a list rather than a single number. Mu Rating, MR,
and the μ in the list are all the same thing.

**σ²** is how settled that estimate is. It is high on a character you have barely played and falls
as you play, and it is what decides which group a character sits in.

## 🖥️ How To Install

Two pieces: the mod that draws the badge, and the helper that feeds it. You need both. Grab them
from [Releases](https://github.com/aarminani/mu-rating-hud/releases), or from
[`download/`](download): `trhmu-0.1.0.zip` is the helper, `TrueProwessMuRating-0.1.0.zip` is the mod.

**1. The mod.** Copy all three files into `...\TEKKEN 8\Polaris\Content\Paks\Mods\MuRating\`,
making those folders if they aren't there:

```
A_TrueProwessMuRating_1000_P.pak
A_TrueProwessMuRating_1000_P.ucas
A_TrueProwessMuRating_1000_P.utoc
```

Keep all three together and don't rename them. On the Tekken Resource Hub Mod Manager, just drop
them in.

**2. The helper.** Unzip anywhere and run `trhmu.exe`. No installer. It isn't code signed
yet, so Windows shows a blue warning the first time: **More info**, then **Run anyway**.

**3. Connect.** Enter your Tekken ID, the code the game's Replay menu shows (`4Rh7-ai3D-3G2a`). It
pairs to the Steam account you're signed in with and connects by itself from then on.

**4. Play.** The helper opens with Tekken, and your MR sits beside your rank.

Needs Windows 10 or later, TEKKEN 8 on Steam, and Microsoft Edge WebView2 (already on almost every
Windows machine). No badge? **Settings → Troubleshooting → Open Mod Folder** shows whether the game
can see the mod.

## 💬 If You Have Any Problems

Join my Discord, find the **mu-support thread**, and file a ticket there:

https://discord.com/invite/DmPqYPtgKg

Attach your diagnostics: **Settings → Troubleshooting → Diagnostics** copies a report of the app,
game, mods, connection, MR and recent log. Pasting that with your ticket is the difference between
a guess and a fix, please make sure to include that!

## ☕ Support The Project

Keep the project alive and growing! As more players join the leaderboard, our server costs rise and
we'd like to continue rolling out new features! If you love tracking your true character skill and want to help maintain
server stability and support ongoing development, please consider donating via our Ko-fi page.
Your contributions directly keep the database fast, the mod responsive, and new updates rolling out!

https://ko-fi.com/trh

## Official Repository Details

The Tekken Resource Hub Mu Rating HUD shows Wavu Wank Mu Ratings in Tekken 8: your MR on the
in game badge, the change after each fight, and your opponents' ratings in the replay menu.

It's two critical things. A badge that draws in game, and a small Windows helper that keeps the badge fed,
so the number beside your rank is the one Wavu has rather than one you have to go and look up.

**Status:** beta. Version 0.1.0. What changed, release by release, is in [CHANGELOG.md](CHANGELOG.md).

This repository is **source available, not open source**. You're welcome to read and review the
code, but you may not redistribute it or reuse it elsewhere. See [LICENSE](LICENSE).

## What's included

- `app/`: the Windows helper (Tauri, Rust and React). It reads ratings, keeps the badge's save slot
  current, and shows ratings, sessions and achievements.
- `feed-worker/`: the Cloudflare Worker that polls Wavu Wank once a minute and publishes a small,
  cached feed. Helpers read that feed instead of asking Wavu themselves.

The in-game badge mod is not part of this repository.

## Accounts

The helper follows one account at a time, the one with the check mark, and keeps up to five
saved.

- **Alts.** A second Tekken ID is one click away, and switching is instant because both are already
  paired to their Steam accounts.
- **Rivals.** Nothing says the account has to be yours. Save a rival's Tekken ID and the ratings
  list, the pages and the feed are theirs; the badge writes for whoever the HUD is following.
- **Auto-connect** follows the Steam account signed in to Tekken. Sign in as your alt and the
  helper moves with you without being asked.
- **Tekken starting opens the helper**, always, and it connects on its own. Headless Mode is the one
  exception: it stays out of sight and connects anyway.
- **Fighter Select** is the other way to pick: a card each, with the character's portrait, instead
  of a list. It is a setting, and the list is still there with it off.

An account is added by its Tekken ID, the same one the game's Replay menu shows, typed in groups of
four. The helper reads that player's Wavu page once to pair it, then leaves Wavu alone.

## The Mini Pages

Under the logo, four pages turn by themselves every five seconds. The arrows and the dots move
between them, and hovering or focusing one stops the clock until you leave.

- **Accounts.** Every saved account with its monogram, the one being followed marked, the rest a
  click away, and Add while there is room for a sixth.
- **Trend.** A line of the last twenty battles on the character you are playing, with that
  character's μ under it and what the twenty added up to. Switch character mid-session and the line
  follows you.
- **Next milestone.** The next hundred, how far you are through it, how much is left, and roughly
  how many wins that is, worked out from what your own recent wins have actually been worth, not
  from an average.
- **This session.** The net MR, the last eight results as squares, the win-loss count and the
  current streak. Beside the pages, the last battle reads `vs Name (Jin) +12 · 3 min ago`.

## Ratings

The main list is every character Wavu has a rating for on the account, in Wavu's own groups. A row
flashes as its rating changes, so a match that has just landed is visible without hunting for it.
Drag the window wider and three more columns appear, in Wavu's order: σ², games, and the date the
character was last played. The size is remembered.

| Group | Wavu's condition | What it means |
| --- | --- | --- |
| Leaderboard | σ² < 75 | Settled. This is the rating Wavu ranks you with. |
| Unqualified | σ² < 110 | A real rating, not yet confident enough to be ranked. |
| Provisional | σ² ≥ 110 | Still being worked out, the first games on a character. |

The groups, the thresholds and the numbers are Wavu's. The app reads the labels off your page
rather than deciding any of it.

Every rating in this project comes from [Wavu Wank](https://wank.wavu.wiki/), built and run by
[6weetbix](https://x.com/6weetbix). None of this exists without that site: it rates every ranked
battle in Tekken 8 and publishes the result for free. Thank you.

## Feed & Achievements

### The feed

A record of what happened, newest first, under day headings, kept per account and five hundred rows
deep.

- **Battles.** Every ranked match: the opponent, their character, yours, the change, and the MR it
  left you on.
- **Awards.** Every achievement and milestone as it was earned, in its rarity's colour, in the
  place in the day it happened.

The first time an account connects it reads a day of battles at once. That first pass fills the
counters in silence and earns nothing, so connecting never dumps a hundred awards into the feed.

### Achievements

Seventy-five of them, in six sections: **Milestones** (10), **Match** (14), **Streaks** (13),
**Daily streaks** (7), **Session** (27) and **Roster** (4).

Milestones are numbers you pass, a peak, a hundred, a games count, and are earned once. The rest
are things you did, and most can be earned again in a later session.

Each one is **Common**, **Uncommon**, **Rare**, **Epic** or **Legendary**, and the rarity is the
colour it is drawn in everywhere: the card, the feed row, the tile and the unread dot. A tiered
achievement carries the rarity of the step you reached, so the same entry gets rarer as you climb
it.

Where several would fire at once, only the biggest does. Beating someone 800 MR above you is an
upset, a giant slaying and a dragon slaying at the same time; Dragon Slayer is what you get, and
the smaller ones are quietly marked as reached.

### Notifications

Earning something puts a card on screen: the rarity, the kind, the title and a sentence naming the
opponent or the number that did it. **Notifications Over the Game** keeps that card above Tekken in
Borderless or Windowed. Nothing draws over exclusive Fullscreen, in any app.

While something earned has not been looked at, a dot sits on the Achievements button in the
rarity's colour.

### Finding Things

The catalog is everything there is to earn, with progress on the ones you haven't.

- **Search** across titles and conditions.
- **Sort** by section, name or rarity, either direction.
- **Earned** filters to what you have. Searching or sorting flattens the six sections into one list,
  because two hits split across six headings are two hits you have to go looking for.
- **Grid or list.** The grid is to glance at; the list is to read. The choice is remembered, and
  clicking a tile in the grid opens the list at it.

## Settings

### HUD

| Setting | What it does |
| --- | --- |
| **Enable MR** | The badge in game. Off, the helper stops writing and the badge shows nothing. |
| **Auto-connect** | Follow the Steam account signed in to Tekken, no Connect click needed. |

### Notifications

| Setting | What it does |
| --- | --- |
| **Hide Notifications** | No cards on screen. Everything is still earned and still in the feed. |

### App

| Setting | What it does |
| --- | --- |
| **Auto-hide Helper** | Puts the window away ten seconds after you stop using it. Moving the pointer back cancels it. |
| **Start with Windows** | Keeps the helper ready in the tray, so it opens and connects the moment Tekken starts. |
| **Fighter Select Style** | Pick accounts from cards instead of a list. |
| **Page Auto Scroll** | Turns the pages under the logo every 5 seconds. |
| **Notifications Over the Game** | Keeps them above Tekken. Needs Borderless or Windowed, nothing draws over exclusive Fullscreen. |
| **Headless Mode** | Silences notifications, starts with Windows and forces Auto-connect. The tray icon greys out; clicking it or launching the helper again opens the window, which turns Headless Mode off. |

### Troubleshooting

| Tool | What it does |
| --- | --- |
| **Open Mod Folder** | Opens the game's Paks folder, and says so when the badge mod isn't in it. |
| **Wavu Wank Table** | Opens the save folder holding `MuRating.sav`. This disconnects from Wavu Wank until you reconnect. |
| **Test Notification** | Fires an achievement card after a short wait, so you can switch to the game first. Five presses show every rarity. |
| **Diagnostics** | Copies a full report of the app, game, mods, connection, MR and recent log, to paste into a Discord bug report. |

Under those, **Changelog**: the same notes as [CHANGELOG.md](CHANGELOG.md), shipped inside the build
so they describe the version actually running.

## Where the ratings come from

Every helper reading Wavu directly would mean thousands of requests a minute for the same public
data. So:

- One Cloudflare Worker asks Wavu for new ranked battles once a minute and republishes them as
  small cached files. Every helper reads that feed, and Wavu sees one reader instead of a crowd.
- Your own player page is read when you connect, and after your own matches at most once every ten
  minutes.
- The helper never touches Tekken's own online API, and it never writes anything to Wavu.

## Building the helper

Requires Rust, Node.js and the Tauri prerequisites for Windows.

```
cd app
npm install
npm run app:build
```

Character portraits are Bandai Namco's art and are not included. Without them the app shows letter
monograms; `app/scripts/portraits_hd.py` makes them from your own Tekken 8 install.

## Credits

Ratings come from [Wavu Wank](https://wank.wavu.wiki/) by 6weetbix. Built for
[tekkenresourcehub.com](https://tekkenresourcehub.com/). Not affiliated with or endorsed by Bandai
Namco Entertainment or Wavu Wank.
