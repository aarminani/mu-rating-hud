# Changelog

The same notes ship inside the helper, under Settings, so they always describe the version actually
running.

## Version 0.1.1 - September 19th, 2026

Fixes for the helper getting in front of the game.

### Fixed

- Notifications could take focus away from Tekken. A card that arrived just as the last one was
  closing was brought back in a way Windows treats as switching to it, so in Borderless or Windowed
  the game lost your inputs for a moment. Notification cards now cannot take focus at all.
- The helper could open itself over a match. It opens when Tekken starts, and one failed check on
  whether Tekken was running, followed by a good one, looked like Tekken restarting. It now needs
  Tekken to be really gone first, and never counts the same game as a new launch.

## Version 0.1.0 - September 17th, 2026

First release.

### In game

- Your Mu Rating on the badge. The helper reads your ratings from Wavu Wank and keeps the badge's
  save slot current, so the number beside your rank is the one Wavu has.
- Your MR change after a ranked match, on screen when the match ends rather than whenever Wavu is
  next read.
- Both sides' ratings in the replay menu. A replay is matched by name and Tekken Power, so the
  rating each player carried into that battle is the one shown.

### Accounts

- Up to five accounts. Alts, a second Tekken ID, or a rival you want to watch; the HUD follows the
  one with the check mark and the rest stay a click away.
- Auto-connect follows the Steam account signed in to Tekken, so switching accounts needs nothing
  from you.
- Tekken starting opens the helper and it connects on its own. In Headless Mode it stays out of
  sight and connects anyway.
- Fighter Select, another way to pick an account: a card each with the character's portrait, instead
  of a list.

### Pages

- Four pages under the logo, turning by themselves every five seconds, or by the arrows either side.
- The trend page graphs the last twenty battles on the character you are playing, and follows you
  when you switch character.
- The milestone page counts the MR left to your next hundred and estimates the wins it takes, from
  what your own recent wins have actually been worth.
- The session page keeps wins, losses, the net change, the last eight results and the current
  streak, with the last opponent and what they cost you.

### Ratings

- The ratings list is every character Wavu has a rating for, in Wavu's own groups: Leaderboard,
  Unqualified and Provisional. A character's row flashes with its change as it lands.
- Drag the window wider and σ², games and the last played date appear, in Wavu's column order. The
  size is remembered.

### Feed and achievements

- A feed of everything that happened: every battle with the opponent, the character, the change and
  the MR it left you on, and every award as it is earned. Five hundred rows an account.
- Seventy-five achievements and milestones across six sections, each one Common, Uncommon, Rare,
  Epic or Legendary.
- An achievement card over the game as you earn one, in the rarity's colour. Notifications Over the
  Game keeps it above Tekken in Borderless or Windowed; nothing draws over exclusive Fullscreen.
- A dot on the Achievements button while something earned has not been looked at.
- The catalog searches, sorts and filters to what you have earned, as a grid to glance at or a list
  to read.

### The app

- Auto-hide puts the window away ten seconds after you stop using it. Moving the pointer back
  cancels it.
- Headless Mode: no window, no notifications, a greyed tray icon, and it starts with Windows and
  connects. Opening the helper again turns it off.

### Troubleshooting

- Diagnostics copies a full report of the app, game, mods, connection, MR and recent log, to paste
  into a Discord bug report.
- Test Notification fires an achievement card after a short wait, so you can switch to the game
  first. Five presses show every rarity.

### Where the ratings come from

- Ratings come from a feed published once a minute rather than from Wavu directly, so one request
  serves every player. Your own page is read when you connect, and at most once every ten minutes
  after your own matches.
