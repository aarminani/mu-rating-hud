export function withEmphasis(text: string, emphasis: string | null | undefined) {
  if (!emphasis) return text;
  const at = text.indexOf(emphasis);
  if (at < 0) return text;
  return (
    <>
      {text.slice(0, at)}
      <strong>{emphasis}</strong>
      {text.slice(at + emphasis.length)}
    </>
  );
}
