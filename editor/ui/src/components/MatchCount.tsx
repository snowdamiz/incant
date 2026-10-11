/**
 * "4 matches"; from 10,000 on, a compact "99K matches" that leaves the filter text room
 * in a narrow column, with the exact count for assistive tech and the tooltip.
 */
export function MatchCount({ count }: { count: number }) {
  const exact = `${count.toLocaleString()} ${count === 1 ? 'match' : 'matches'}`;
  if (count < 10_000) return <>{exact}</>;
  const compact = new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(count);
  return (
    <span title={exact}>
      <span aria-hidden="true">{compact} matches</span>
      <span className="visually-hidden">{exact}</span>
    </span>
  );
}
