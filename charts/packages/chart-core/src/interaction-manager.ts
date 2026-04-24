import type {
  ChartInteractionCommand,
  ChartInteractionStyle,
  InteractionCapabilities,
  InteractionEvent,
  InteractionSession,
  InteractionState,
  InteractionTarget,
  InteractionTrigger,
  PrimaryInteractionMode,
  SecondaryInteractionModifier
} from "@ophiolite/charts-data-models";
import { resolveInteractionBinding } from "./interaction-style";

type Listener = (event: InteractionEvent) => void;

export class InteractionManager {
  private readonly listeners = new Set<Listener>();
  private state: InteractionState;
  private style: ChartInteractionStyle | null;

  constructor(
    capabilities: InteractionCapabilities,
    primaryMode: PrimaryInteractionMode,
    modifiers: SecondaryInteractionModifier[] = [],
    style: ChartInteractionStyle | null = null
  ) {
    this.state = {
      capabilities,
      primaryMode,
      modifiers: uniqueModifiers(modifiers.filter((modifier) => capabilities.modifiers.includes(modifier))),
      focused: false,
      hoverTarget: null,
      session: null
    };
    this.style = style;
  }

  getState(): InteractionState {
    return {
      capabilities: {
        primaryModes: [...this.state.capabilities.primaryModes],
        modifiers: [...this.state.capabilities.modifiers]
      },
      primaryMode: this.state.primaryMode,
      modifiers: [...this.state.modifiers],
      focused: this.state.focused,
      hoverTarget: this.state.hoverTarget ? { ...this.state.hoverTarget } : null,
      session: cloneSession(this.state.session)
    };
  }

  on(listener: Listener): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  setPrimaryMode(mode: PrimaryInteractionMode): void {
    if (!this.state.capabilities.primaryModes.includes(mode) || this.state.primaryMode === mode) {
      return;
    }
    this.cancelSession();
    this.state.primaryMode = mode;
    this.emit({ type: "modeChange", primaryMode: mode });
  }

  enableModifier(modifier: SecondaryInteractionModifier): void {
    if (!this.state.capabilities.modifiers.includes(modifier) || this.state.modifiers.includes(modifier)) {
      return;
    }
    this.state.modifiers = uniqueModifiers([...this.state.modifiers, modifier]);
    this.emit({ type: "modifierChange", modifier, enabled: true });
  }

  disableModifier(modifier: SecondaryInteractionModifier): void {
    if (!this.state.modifiers.includes(modifier)) {
      return;
    }
    this.state.modifiers = this.state.modifiers.filter((candidate) => candidate !== modifier);
    this.emit({ type: "modifierChange", modifier, enabled: false });
  }

  toggleModifier(modifier: SecondaryInteractionModifier): void {
    if (this.state.modifiers.includes(modifier)) {
      this.disableModifier(modifier);
    } else {
      this.enableModifier(modifier);
    }
  }

  setFocused(focused: boolean): void {
    if (this.state.focused === focused) {
      return;
    }
    this.state.focused = focused;
    this.emit({ type: "focusChange", focused });
  }

  setHoverTarget(target: InteractionTarget | null): void {
    const previous = this.state.hoverTarget;
    if (sameTarget(previous, target)) {
      return;
    }
    this.state.hoverTarget = target ? { ...target } : null;
    this.emit({ type: "hoverTargetChange", target: this.state.hoverTarget ? { ...this.state.hoverTarget } : null });
  }

  beginSession(session: InteractionSession): void {
    this.state.session = cloneSession(session);
    switch (session.kind) {
      case "topEdit":
        this.emit({ type: "topEditStart", session: cloneSession(session) as any });
        break;
      case "lasso":
        this.emit({ type: "lassoStart", session: cloneSession(session) as any });
        break;
      case "zoomRect":
        this.emit({ type: "zoomRectStart", session: cloneSession(session) as any });
        break;
    }
  }

  updateSession(session: InteractionSession): void {
    this.state.session = cloneSession(session);
    switch (session.kind) {
      case "topEdit":
        this.emit({ type: "topEditPreview", session: cloneSession(session) as any });
        break;
      case "lasso":
        this.emit({ type: "lassoPreview", session: cloneSession(session) as any });
        break;
      case "zoomRect":
        this.emit({ type: "zoomRectPreview", session: cloneSession(session) as any });
        break;
    }
  }

  commitSession(): void {
    if (!this.state.session) {
      return;
    }
    const session = cloneSession(this.state.session)!;
    this.state.session = null;
    switch (session.kind) {
      case "topEdit":
        this.emit({ type: "topEditCommit", session: session as any });
        break;
      case "lasso":
        this.emit({ type: "lassoComplete", session: session as any });
        break;
      case "zoomRect":
        this.emit({ type: "zoomRectCommit", session: session as any });
        break;
    }
  }

  cancelSession(): void {
    if (!this.state.session) {
      return;
    }
    const session = cloneSession(this.state.session)!;
    this.state.session = null;
    switch (session.kind) {
      case "topEdit":
        this.emit({ type: "topEditCancel", session: session as any });
        break;
      case "lasso":
        this.emit({ type: "lassoCancel", session: session as any });
        break;
      case "zoomRect":
        this.emit({ type: "zoomRectCancel", session: session as any });
        break;
    }
  }

  hasModifier(modifier: SecondaryInteractionModifier): boolean {
    return this.state.modifiers.includes(modifier);
  }

  getStyle(): ChartInteractionStyle | null {
    return this.style
      ? {
          ...this.style,
          manipulators: [...this.style.manipulators],
          bindings: this.style.bindings.map((binding) => ({ ...binding }))
        }
      : null;
  }

  setStyle(style: ChartInteractionStyle | null): void {
    this.style = style
      ? {
          ...style,
          manipulators: [...style.manipulators],
          bindings: style.bindings.map((binding) => ({ ...binding }))
        }
      : null;
  }

  resolveTriggerCommand(trigger: InteractionTrigger): ChartInteractionCommand | null {
    const binding = resolveInteractionBinding(this.style, trigger);
    if (!binding) {
      return null;
    }
    return {
      type: "semanticAction",
      command: binding.command
    };
  }

  private emit(event: InteractionEvent): void {
    for (const listener of this.listeners) {
      listener(event);
    }
  }
}

function uniqueModifiers(modifiers: SecondaryInteractionModifier[]): SecondaryInteractionModifier[] {
  return [...new Set(modifiers)];
}

function sameTarget(left: InteractionTarget | null, right: InteractionTarget | null): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function cloneSession(session: InteractionSession | null): InteractionSession | null {
  return session ? JSON.parse(JSON.stringify(session)) as InteractionSession : null;
}
