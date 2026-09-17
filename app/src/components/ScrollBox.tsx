import { useEffect, useRef, type ReactNode } from "react";

const INSET = 4;
const MIN_THUMB = 24;

export function ScrollBox({ className, children }: { className: string; children: ReactNode }) {
  const listRef = useRef<HTMLDivElement | null>(null);
  const laneRef = useRef<HTMLDivElement | null>(null);
  const thumbRef = useRef<HTMLDivElement | null>(null);
  const geo = useRef({ top: 0, height: 0, on: false });

  const measure = () => {
    const el = listRef.current;
    const lane = laneRef.current;
    const thumb = thumbRef.current;
    if (!el || !lane || !thumb) return;
    const { scrollTop, scrollHeight, clientHeight } = el;
    const on = scrollHeight > clientHeight + 1;
    el.classList.toggle("scrollable", on);
    lane.hidden = !on;
    thumb.hidden = !on;
    geo.current.on = on;
    if (!on) return;
    const travel = clientHeight - INSET * 2;
    const height = Math.max(MIN_THUMB, (travel * clientHeight) / scrollHeight);
    const top = el.offsetTop + el.clientTop + INSET + ((travel - height) * scrollTop) / (scrollHeight - clientHeight);
    geo.current.top = top;
    geo.current.height = height;
    lane.style.top = `${el.offsetTop + el.clientTop}px`;
    lane.style.height = `${clientHeight}px`;
    thumb.style.top = `${top - INSET}px`;
    thumb.style.height = `${height + INSET * 2}px`;
  };

  useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    const watchChildren = () => Array.from(el.children).forEach((c) => ro.observe(c));
    watchChildren();
    const mo = new MutationObserver(() => {
      watchChildren();
      measure();
    });
    mo.observe(el, { childList: true, subtree: true });
    return () => {
      ro.disconnect();
      mo.disconnect();
    };
  }, []);

  const wheel = (e: React.WheelEvent) => {
    if (listRef.current) listRef.current.scrollTop += e.deltaY;
  };

  const onThumbDown = (e: React.PointerEvent<HTMLDivElement>) => {
    const el = listRef.current;
    const thumb = e.currentTarget;
    if (!el || e.button !== 0) return;
    e.preventDefault();
    thumb.setPointerCapture(e.pointerId);
    thumb.classList.add("active");
    document.body.classList.add("scroll-dragging");
    const startY = e.clientY;
    const startScroll = el.scrollTop;
    const range = el.scrollHeight - el.clientHeight;
    const travel = el.clientHeight - INSET * 2 - geo.current.height;
    const move = (ev: PointerEvent) => {
      el.scrollTop = startScroll + ((ev.clientY - startY) * range) / Math.max(1, travel);
    };
    const up = () => {
      thumb.classList.remove("active");
      document.body.classList.remove("scroll-dragging");
      thumb.removeEventListener("pointermove", move);
      thumb.removeEventListener("pointerup", up);
      thumb.removeEventListener("pointercancel", up);
    };
    thumb.addEventListener("pointermove", move);
    thumb.addEventListener("pointerup", up);
    thumb.addEventListener("pointercancel", up);
  };

  const onLaneDown = (e: React.PointerEvent<HTMLDivElement>) => {
    const el = listRef.current;
    if (!el || e.button !== 0 || e.target !== e.currentTarget) return;
    const y = e.nativeEvent.offsetY + el.offsetTop + el.clientTop;
    el.scrollBy({ top: (y < geo.current.top ? -1 : 1) * el.clientHeight * 0.9 });
  };

  return (
    <div className="scrollbox">
      <div ref={listRef} className={`${className} overlay-scroll`} onScroll={measure}>
        {children}
      </div>
      <div ref={laneRef} className="scroll-lane" hidden onWheel={wheel} onPointerDown={onLaneDown} aria-hidden />
      <div ref={thumbRef} className="scroll-thumb" hidden onWheel={wheel} onPointerDown={onThumbDown} aria-hidden>
        <span className="scroll-pill" />
      </div>
    </div>
  );
}
