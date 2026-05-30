import type * as Y from 'yjs';
import { BaseProvider } from './base';

const TEXT_ROOT = 'content';

export class ChannelDocProvider extends BaseProvider {
  readonly text: Y.Text;

  constructor(channelId: string) {
    super({
      prefix: 'channel_doc',
      idField: 'channel_id',
      id: channelId,
      awarenessStateType: 'channel_doc_awareness_state',
      awarenessUpdateType: 'channel_doc_awareness_update',
    });
    this.text = this.doc.getText(TEXT_ROOT);
  }

  replaceText(next: string): void {
    this.doc.transact(() => {
      this.text.delete(0, this.text.length);
      this.text.insert(0, next);
    });
  }
}
