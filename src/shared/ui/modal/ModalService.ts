// Questions and messages over the window (quit?, delete?, about): one at a time, answered with a
// promise. Replaces the Python windows' message boxes.
import { Store } from '../../lib/Store.ts';

export interface ModalButton {
  label: string;
  value: boolean;
  accent?: boolean;
}

export interface ModalState {
  open: boolean;
  text: string;
  buttons: ModalButton[];
}

export class ModalService {
  readonly store = new Store<ModalState>({ open: false, text: '', buttons: [] });
  private resolve: ((v: boolean) => void) | null = null;

  private show(text: string, buttons: ModalButton[]): Promise<boolean> {
    this.resolve?.(false);
    this.store.set({ open: true, text, buttons });
    return new Promise((r) => {
      this.resolve = r;
    });
  }

  confirm(text: string, yes: string, no: string): Promise<boolean> {
    return this.show(text, [{ label: no, value: false }, { label: yes, value: true, accent: true }]);
  }

  alert(text: string, ok: string): Promise<boolean> {
    return this.show(text, [{ label: ok, value: true, accent: true }]);
  }

  answer(value: boolean) {
    this.store.set({ open: false });
    const r = this.resolve;
    this.resolve = null;
    r?.(value);
  }
}
