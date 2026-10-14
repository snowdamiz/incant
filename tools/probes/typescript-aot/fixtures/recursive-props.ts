// Exact recursive data shape permitted by NativeProp; no engine imports needed.
type Prop = null | boolean | number | string | readonly Prop[] | { readonly [key: string]: Prop };
export function probe(): number {
  const props: { readonly [key: string]: Prop } = { speed: 3, nested: [true, null] };
  return typeof props.speed === 'number' ? props.speed : -1;
}
