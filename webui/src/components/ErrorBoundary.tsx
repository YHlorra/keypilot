import * as React from "react";
import { Button } from "./ui/button";

interface ErrorBoundaryProps {
  children: React.ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  handleRetry = () => {
    this.setState({ hasError: false, error: null });
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex items-center justify-center h-screen bg-background">
          <div className="rounded-md border border-border bg-card p-6 max-w-lg w-full">
            <h2 className="text-lg font-semibold text-danger">出错了</h2>
            <p className="text-sm text-muted-foreground break-all mt-2">
              {this.state.error?.message || "发生了未知错误"}
            </p>
            {this.state.error?.stack && (
              <pre className="text-xs bg-surface border border-border rounded p-3 overflow-auto max-h-60 mt-3">
                {this.state.error.stack}
              </pre>
            )}
            <div className="mt-4 flex justify-center">
              <Button
                onClick={this.handleRetry}
                className="rounded-full px-3 py-1.5"
                size="sm"
                variant="default"
              >
                重试
              </Button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
