import { Fragment } from 'react';
import type { ReactNode } from 'react';

/**
 * A path or message that may wrap after each "/" rather than mid-name. `<wbr>` adds a
 * break opportunity without changing the text that assistive technology reads.
 */
export function breakAfterSlashes(text: string): ReactNode {
  const parts = text.split('/');
  return parts.map((part, index) => (
    <Fragment key={index}>
      {part}
      {index < parts.length - 1 ? (
        <>
          /<wbr />
        </>
      ) : null}
    </Fragment>
  ));
}
