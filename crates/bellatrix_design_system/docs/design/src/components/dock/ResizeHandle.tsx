import * as React from 'react';

/** Visual-layout splitter only: no diagram editing behavior. */
export class ResizeHandle extends React.Component<{
  axis: 'horizontal' | 'vertical';
  label: string;
  onDelta: (delta: number) => void;
  onReset: () => void;
  className?: string;
  value?: number;
  min?: number;
  max?: number;
}> {
  private handle: HTMLDivElement | null = null;
  private previous = 0;
  private dragging = false;

  componentDidMount() {
    this.handle?.addEventListener('pointerdown', this.pointerDown);
  }
  componentWillUnmount() {
    this.handle?.removeEventListener('pointerdown', this.pointerDown);
    this.endDrag();
  }
  private coordinate(event: PointerEvent) {
    return this.props.axis === 'horizontal' ? event.clientX : event.clientY;
  }
  private pointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    event.preventDefault();
    this.dragging = true;
    this.previous = this.coordinate(event);
    document.body.classList.add('is-resizing');
    window.addEventListener('pointermove', this.pointerMove);
    window.addEventListener('pointerup', this.pointerUp);
    window.addEventListener('pointercancel', this.pointerUp);
  };
  private pointerMove = (event: PointerEvent) => {
    if (!this.dragging) return;
    const coordinate = this.coordinate(event);
    const delta = coordinate - this.previous;
    this.previous = coordinate;
    if (delta !== 0) this.props.onDelta(delta);
  };
  private pointerUp = () => this.endDrag();
  private endDrag() {
    this.dragging = false;
    if (typeof document !== 'undefined') document.body.classList.remove('is-resizing');
    window.removeEventListener('pointermove', this.pointerMove);
    window.removeEventListener('pointerup', this.pointerUp);
    window.removeEventListener('pointercancel', this.pointerUp);
  }
  private keyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const horizontal = this.props.axis === 'horizontal';
    if (event.key === 'Home') { event.preventDefault(); this.props.onReset(); return; }
    const key = horizontal ? ['ArrowLeft', 'ArrowRight'] : ['ArrowUp', 'ArrowDown'];
    if (event.key !== key[0] && event.key !== key[1]) return;
    event.preventDefault();
    this.props.onDelta(event.key === key[1] ? 16 : -16);
  };
  render() {
    return <div
      ref={node => { this.handle = node; }}
      className={`resize-handle resize-${this.props.axis} ${this.props.className || ''}`}
      role="separator" aria-orientation={this.props.axis === 'horizontal' ? 'vertical' : 'horizontal'}
      aria-label={`${this.props.label}. Drag to resize; Home resets.`}
      aria-valuenow={this.props.value} aria-valuemin={this.props.min} aria-valuemax={this.props.max}
      tabIndex={0} title={`${this.props.label} · drag to resize · double-click to reset`}
      onKeyDown={this.keyDown} onDoubleClick={this.props.onReset}
    />;
  }
}
