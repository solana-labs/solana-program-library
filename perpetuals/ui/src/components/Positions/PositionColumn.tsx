export const COL_WIDTHS = {
  1: 14,
  2: 13,
  3: 13,
  4: 13,
  5: 13,
  6: 13,
  7: 18,
} as const;

export function PositionColumn(props: {
  children: React.ReactNode;
  num: keyof typeof COL_WIDTHS;
}) {
  return (
    <div style={{ width: `${COL_WIDTHS[props.num]}%` }}>{props.children}</div>
  );
}
