# Native requests from the Claude UI work (handoff 0003)

Live file. Astra: ordered by priority. The UI does not edit editor/bridge.

## N1. Add the provider request shapes to `HostRequest`

`editor/bridge/contract.ts` lists `provider.cancel`, `provider.disconnect` and
`provider.switch` as capabilities, but the `HostRequest` union still has only:

```ts
| { readonly type: 'provider.connect'; readonly method: 'oauth' | 'api-key'; readonly phase?: 'browser' | 'validating' }
```

The UI sends these shapes today (defined locally in `editor/ui/src/bridge/provider.ts`
and cast at one point in `ShellContext.tsx`):

```ts
| { readonly type: 'provider.connect'; readonly method: 'oauth'; readonly accountId?: string; readonly add?: boolean }
| { readonly type: 'provider.cancel' }
| { readonly type: 'provider.disconnect' }
| { readonly type: 'provider.switch'; readonly accountId: string }
```

Requested: add them to `HostRequest`, and drop `phase` from the connect request
(`phase` is state the host reports, not something the UI asks for). Once merged, the
UI deletes its local `ProviderRequest` type and the cast.

Verify: `npm run typecheck --workspace editor/ui` with the local type removed.

## N2. Semantics the UI assumes (please confirm or correct)

1. `provider.connect` with no `accountId` resumes the selected registration. The UI
   sends it for "Continue with ChatGPT", "Open browser again" and retry after an error.
   A repeat connect while already `connecting/browser` should relaunch the system
   browser for the same attempt, not start a second listener.
2. `provider.disconnect` signs out the **active** account only, on this computer, and
   leaves other saved accounts listed. The confirmation copy says exactly that.
3. `provider.switch` to a signed-out registration may move to `connecting/browser`.
4. After `provider.cancel`, the host publishes `not-connected` (keeping `accounts`).
5. `ProviderState.activeAccount` is set whenever `accounts` is non-empty, so the UI
   can mark which account is active even while signed out.
6. `error.message` and `message` are safe to show verbatim. The UI displays them as
   is, with `error.code` on a second line.
7. The snapshot starts in `checking` and moves to a final state without a request.

## N3. Native verification needed

Browser fixtures are not native evidence. With a build that advertises `provider.*`,
please capture the Incant window only in: checking at launch, signed out,
browser wait (system browser opened), validating, connected after an app restart,
and an error such as browser launch failure. No account consent is needed for any
state except connected; the director signs in.
