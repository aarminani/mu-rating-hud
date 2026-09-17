import type { CharRating } from "../lib/types";
import { grouped, shortDate, signed } from "../lib/format";
import { ScrollBox } from "./ScrollBox";

export function RatingsList({ ratings }: { ratings: CharRating[] }) {
  return (
    <ScrollBox className={`ratings rating-list${ratings.some((r) => r.change) ? " with-changes" : ""}`}>
      {grouped(ratings).map(([group, rows]) => (
        <div key={group}>
          <div className="group-head">
            <span className="group-word">{group.split("(")[0].trim()}</span>
            {group.includes("(") && <span className="group-cond">({group.split("(")[1]}</span>}
            <span className="faint">&middot; {rows.length}</span>
          </div>
          {rows.map((r) => (
            <div
              className={`row${r.change ? " changed" : ""}${r.change && r.change < 0 ? " down" : ""}`}
              key={`${r.character}:${r.last_seen ?? ""}`}
            >
              <span className="name">{r.character}</span>
              <span className="mu-cell">
                &#956; {r.mu}
                <span className={`mu-change${r.change && r.change < 0 ? " down" : " up"}`}>
                  {signed(r.change)}
                </span>
              </span>
              <span className="sub rd-cell">{r.sigma != null ? `σ² ${r.sigma}` : ""}</span>
              <span className="sub games-cell">{r.games ? `${r.games} games` : ""}</span>
              <span className="sub date-cell">{shortDate(r.last_seen)}</span>
            </div>
          ))}
        </div>
      ))}
    </ScrollBox>
  );
}
