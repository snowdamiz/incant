import { describe, expect, it, vi } from "vitest";
import { NativeBridge, snapshotFromEngine } from "./native";
import type { EngineRead, EngineResponse, Invoke } from "./native";
import type { Ulid, WindowChrome } from "./contract";
const scene = "00000000000000000000000002";
const entity = "00000000000000000000000010";
function read(): EngineRead {
  return {
    project: {
      id: "00000000000000000000000001",
      name: "Test",
      scenes: {
        [scene]: {
          id: scene,
          name: "Main",
          entities: {
            [entity]: {
              id: entity,
              name: "Cube",
              parent: null,
              components: {
                Transform: {
                  translation: [0, 0, 0],
                  rotation: [0, 0, 0, 1],
                  scale: [1, 1, 1],
                },
              },
            },
          },
        },
      },
    },
    revision: 7,
    can_redo: false,
    history: [],
    applied: 0,
    schemas: {
      Transform: {
        order: ["translation", "rotation", "scale"],
        properties: {
          translation: {
            type: "array",
            items: { type: "number" },
            minItems: 3,
            maxItems: 3,
          },
        },
      },
    },
    console: [],
    viewport_error: null,
  };
}
describe("native bridge", () => {
  it("reports a missing or rejected event subscription instead of waiting indefinitely", async () => {
    const invoke = vi.fn(async () => read()) as Invoke;
    for (const listen of [undefined, async () => { throw new Error("Event permission denied"); }]) {
      const bridge = new NativeBridge(invoke);
      await bridge.startEngineUpdates(listen);
      expect(bridge.getSnapshot().connection.status).toBe("error");
      expect(bridge.getSnapshot().hierarchy.status).toBe("error");
      expect(bridge.getSnapshot().entities).toEqual({});
    }
    expect(invoke).not.toHaveBeenCalled();
  });
  it("subscribes before reading and ignores an obsolete loading response after completion", async () => {
    let notify: () => void = () => { throw new Error("No listener"); };
    let finishInitial: (value: EngineResponse) => void = () => { throw new Error("No read"); };
    let calls = 0;
    const invoke: Invoke = async <T>() => {
      if (calls++ === 0) return await new Promise<EngineResponse>((resolve) => { finishInitial = resolve; }) as T;
      return read() as T;
    };
    const bridge = new NativeBridge(invoke);
    const starting = bridge.startEngineUpdates(async (event, listener) => {
      expect(event).toBe("incant:engine-changed");
      expect(calls).toBe(0);
      notify = () => listener({ payload: null as never });
      return () => undefined;
    });
    await vi.waitFor(() => expect(calls).toBe(1));
    expect(bridge.getSnapshot().connection.status).toBe("connecting");
    expect(bridge.getSnapshot().hierarchy.status).toBe("loading");
    expect((await bridge.dispatch({ type: "history.undo" })).ok).toBe(false);
    notify();
    await vi.waitFor(() => expect(bridge.getSnapshot().connection.status).toBe("ready"));
    finishInitial({ status: "loading" });
    await starting;
    expect(bridge.getSnapshot().hierarchy.status).toBe("ready");
    expect(calls).toBe(2);
  });
  it("removes stale project state on load or transport failure while preserving account metadata", async () => {
    let response: EngineResponse = read();
    let transportError = false;
    const calls: string[] = [];
    const invoke: Invoke = async <T>(command: string) => {
      calls.push(command);
      if (transportError) throw "Native transport unavailable";
      return response as T;
    };
    const bridge = new NativeBridge(invoke);
    const provider = { status: "connected", provider: "openai", method: "oauth", accountLabel: "Test account" } as const;
    bridge.updateProvider(provider);
    await bridge.start();
    expect(bridge.getSnapshot().entities[entity]?.name).toBe("Cube");
    const error = { code: "project.timeout", message: "Project loading timed out." };
    response = { status: "error", error };
    await bridge.start();
    expect(bridge.getSnapshot().connection).toEqual({ status: "error", error });
    expect(bridge.getSnapshot().hierarchy).toEqual({ status: "error", error });
    expect(bridge.getSnapshot().entities).toEqual({});
    expect(bridge.getSnapshot().diagnostics[0]?.message).toBe(error.message);
    expect(bridge.getSnapshot().provider).toEqual(provider);
    expect((await bridge.dispatch({ type: "history.undo" })).ok).toBe(false);
    expect(calls).toEqual(["engine_read", "engine_read"]);
    response = read();
    await bridge.start();
    transportError = true;
    await bridge.start();
    expect(bridge.getSnapshot().connection.status).toBe("error");
    expect(bridge.getSnapshot().entities).toEqual({});
    expect((await bridge.dispatch({ type: "entity.rename", entity: entity as Ulid, name: "Stale" })).ok).toBe(false);
    expect(bridge.getSnapshot().provider).toEqual(provider);
    expect(calls).toHaveLength(4);
  });
  it("forwards validated native history intent to the UI without mutating the document", async () => {
    const ipc: string[] = [];
    const invoke: Invoke = async <T>(command: string) => { ipc.push(command); return read() as T; };
    const bridge = new NativeBridge(invoke);
    let send: (payload: unknown) => void = () => { throw new Error('No menu listener'); };
    await bridge.startHistoryRequests(async (name, listener) => {
      expect(name).toBe('incant:history-request');
      send = (payload) => listener({ payload: payload as never });
      return () => undefined;
    });
    const intents: string[] = [];
    const unsubscribe = bridge.subscribeHistoryRequests((action) => intents.push(action));
    for (const payload of ['undo', 'redo', 'delete', null, { action: 'undo' }]) send(payload);
    expect(intents).toEqual(['undo', 'redo']);
    expect(ipc).toEqual([]);
    unsubscribe();
    send('undo');
    expect(intents).toEqual(['undo', 'redo']);
  });
  it("projects the Rust scene hierarchy and component schema without inventing capabilities", () => {
    const value = snapshotFromEngine(read());
    expect(value.hierarchy.status).toBe("ready");
    if (value.hierarchy.status !== "ready") throw new Error("not ready");
    expect(value.hierarchy.value.roots).toEqual([scene]);
    expect(value.hierarchy.value.nodes[entity]?.parent).toBe(scene);
    expect(value.entities[entity]?.components[0]?.type).toBe("Transform");
    expect(value.entities[entity]?.kind).toBe("entity");
    expect(value.schemas.Transform?.order).toEqual(["translation", "rotation", "scale"]);
    expect(value.agent.status).toBe("unavailable");
  });
  it("sends a rename and observed revision through IPC, then uses only the returned engine state", async () => {
    const calls: {
      command: string;
      args: Record<string, unknown> | undefined;
    }[] = [];
    const invoke: Invoke = async <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      calls.push({ command, args });
      const result = read();
      if (command === "engine_execute") {
        result.project.scenes[scene]!.entities[entity]!.name = "Renamed";
        result.revision = 8;
      }
      return result as T;
    };
    const bridge = new NativeBridge(invoke);
    await bridge.start();
    const before = bridge.getSnapshot();
    expect(bridge.getSnapshot()).toBe(before);
    expect(Object.isFrozen(before.entities[entity]?.components[0]?.value)).toBe(
      true,
    );
    expect(
      await bridge.dispatch({
        type: "entity.rename",
        entity: entity as Ulid,
        name: "Renamed",
      }),
    ).toEqual({ ok: true });
    expect(calls[1]?.command).toBe("engine_execute");
    expect(calls[1]?.args?.expectedRevision).toBe(7);
    expect(calls[1]?.args?.commands).toEqual([
      {
        op: "rename_entity",
        scene_id: scene,
        entity_id: entity,
        name: "Renamed",
      },
    ]);
    expect(bridge.getSnapshot().entities[entity]?.name).toBe("Renamed");
    expect(before.entities[entity]?.name).toBe("Cube");
  });
  it("rejects unsupported host requests without issuing IPC", async () => {
    let calls = 0;
    const invoke: Invoke = async <T>() => {
      calls++;
      return read() as T;
    };
    const bridge = new NativeBridge(invoke);
    const result = await bridge.request({
      type: "agent.send",
      text: "anything",
    });
    expect(result.ok).toBe(false);
    expect(calls).toBe(0);
  });
  it("converts viewport bounds to physical pixels without mutating the document", async () => {
    let request: unknown;
    const invoke: Invoke = async <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      request = { command, args };
      return undefined as T;
    };
    const bridge = new NativeBridge(invoke);
    expect(
      await bridge.request({
        type: "viewport.bounds",
        rect: { x: 100, y: 40, width: 500, height: 300 },
        devicePixelRatio: 2,
        cornerRadii: [0, 0, 10, 10],
      }),
    ).toEqual({ ok: true });
    expect(request).toEqual({
      command: "viewport_bounds",
      args: { rect: [200, 80, 1000, 600], cornerRadii: [0, 0, 20, 20] },
    });
  });
  it("keeps engine errors visible and does not apply optimistic document edits", async () => {
    const invoke: Invoke = async <T>(command: string) => {
      if (command === "engine_execute") throw "revision conflict";
      return read() as T;
    };
    const bridge = new NativeBridge(invoke);
    await bridge.start();
    const result = await bridge.dispatch({
      type: "entity.rename",
      entity: entity as Ulid,
      name: "Not applied",
    });
    expect(result.ok).toBe(false);
    expect(bridge.getSnapshot().entities[entity]?.name).toBe("Cube");
  });
  it("routes window actions to the host without changing document history", async () => {
    const calls: unknown[] = [];
    const invoke: Invoke = async <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      calls.push({ command, args });
      return undefined as T;
    };
    const bridge = new NativeBridge(invoke);
    for (const type of [
      "window.drag",
      "window.minimize",
      "window.maximize",
      "window.close",
    ] as const) {
      expect(await bridge.request({ type })).toEqual({ ok: true });
    }
    expect(calls).toEqual(
      ["drag", "minimize", "maximize", "close"].map((action) => ({
        command: "window_action",
        args: { action },
      })),
    );
    expect(bridge.getSnapshot().history.entries).toEqual([]);
  });
  it("preserves native window state across engine snapshots and publishes focus changes", async () => {
    const chrome: WindowChrome = {
      platform: "macos",
      controls: "native-overlay",
      maximized: false,
      fullscreen: false,
      focused: true,
      leadingInset: 78,
      trailingInset: 0,
    };
    const invoke: Invoke = async <T>(command: string) =>
      (command === "window_read" ? chrome : read()) as T;
    const bridge = new NativeBridge(invoke);
    await bridge.startWindowUpdates();
    await bridge.start();
    expect(bridge.getSnapshot().window).toEqual(chrome);
    let notices = 0;
    const unsubscribe = bridge.subscribe(() => {
      notices++;
    });
    bridge.updateWindow({ ...chrome, focused: false });
    expect(notices).toBe(1);
    expect(bridge.getSnapshot().window?.focused).toBe(false);
    expect(bridge.getSnapshot().hierarchy.status).toBe("ready");
    unsubscribe();
  });
  it("restores account metadata and retains it across project edits", async () => {
    const provider = { status: "connected", provider: "openai", method: "oauth", accountLabel: "Test account", activeAccount: "test-id" } as const;
    const invoke: Invoke = async <T>(command: string) => (command === "provider_read" ? provider : read()) as T;
    const bridge = new NativeBridge(invoke);
    await bridge.startProviderUpdates();
    await bridge.start();
    expect(bridge.getSnapshot().provider).toEqual(provider);
    await bridge.dispatch({ type: "entity.rename", entity: entity as Ulid, name: "Changed" });
    expect(bridge.getSnapshot().provider).toEqual(provider);
    expect(JSON.stringify(bridge.getSnapshot().provider)).not.toMatch(/token|authorization_url/i);
  });
  it("routes account selection and cancellation without exposing credentials or writing a project", async () => {
    const calls: unknown[] = [];
    const invoke: Invoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      return { status: "not-connected", provider: "openai" } as T;
    };
    const bridge = new NativeBridge(invoke);
    for (const request of [{ type: "provider.connect", method: "oauth", add: true }, { type: "provider.switch", accountId: "saved" }, { type: "provider.cancel" }, { type: "provider.disconnect" }] as const) {
      expect(await bridge.request(request)).toEqual({ ok: true });
    }
    expect(calls).toContainEqual({ command: "provider_action", args: { action: "connect", accountId: null, add: true } });
    expect(calls).toContainEqual({ command: "provider_action", args: { action: "switch", accountId: "saved", add: false } });
    expect(calls).toHaveLength(8);
    expect(bridge.getSnapshot().history.entries).toEqual([]);
  });

});
