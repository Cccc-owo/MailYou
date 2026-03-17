export type CloseBehaviorPreference = 'ask' | 'always_background' | 'always_quit'
export type CloseRequestAction = 'background' | 'quit'

export interface WindowControlsBridge {
  minimize(): Promise<void>
  toggleMaximize(): Promise<boolean>
  close(): Promise<void>
  isMaximized(): Promise<boolean>
  openExternal(url: string): Promise<void>
  setCloseBehaviorPreference(value: CloseBehaviorPreference): Promise<void>
  resolveCloseRequest(action: CloseRequestAction, rememberBackground: boolean): Promise<void>
  setBackgroundSyncInterval(minutes: number): Promise<void>
  exportPdf(html: string, fileName: string): Promise<boolean>
  openTextFile(
    filters: { name: string; extensions: string[] }[],
  ): Promise<{ content: string; fileName: string } | null>
  saveTextFile(
    content: string,
    suggestedName: string,
    filters: { name: string; extensions: string[] }[],
  ): Promise<boolean>
  onCloseRequested(callback: () => void): () => void
}
