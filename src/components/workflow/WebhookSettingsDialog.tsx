/**
 * Webhook Settings Dialog
 * Dialog for configuring webhook notifications for a workflow
 * Per-workflow server architecture: each workflow has its own HTTP server
 * @see specs/012-workflow-webhook-support
 *
 * Design: Following AIReviewDialog pattern with gradient header, icon badge, and refined UI
 */

import { useState, useEffect, useRef } from 'react';
import {
  Webhook,
  Bell,
  Link,
  AlertCircle,
  CheckCircle,
  Play,
  Plus,
  Trash2,
  FileCode,
  Loader2,
  Copy,
  RefreshCw,
  ArrowDownToLine,
  ArrowUpFromLine,
  X,
} from 'lucide-react';
import { cn } from '../../lib/utils';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Toggle } from '../ui/Toggle';
import { isTopModal, registerModal, unregisterModal } from '../ui/modalStack';
import type { WebhookConfig, WebhookTrigger, WebhookTestResult } from '../../types/webhook';
import type {
  IncomingWebhookConfig,
  IncomingWebhookServerStatus,
} from '../../types/incoming-webhook';
import { DEFAULT_PAYLOAD_TEMPLATE, SUPPORTED_VARIABLES } from '../../types/webhook';
import {
  generateWebhookUrl,
  generateCurlWithSignature,
  DEFAULT_INCOMING_WEBHOOK_PORT,
} from '../../types/incoming-webhook';
import { webhookAPI, incomingWebhookAPI, type PortStatus } from '../../lib/tauri-api';

// Payload format presets
type PayloadFormat = 'discord' | 'slack' | 'telegram' | 'custom';
type TabType = 'outgoing' | 'incoming';

const DISCORD_TEMPLATE = `{
  "content": "{{status}} **{{workflow_name}}** completed in {{duration}}ms"
}`;

const SLACK_TEMPLATE = `{
  "text": "{{status}} *{{workflow_name}}* completed in {{duration}}ms"
}`;

const TELEGRAM_TEMPLATE = `{
  "chat_id": "YOUR_CHAT_ID",
  "text": "{{status}} *{{workflow_name}}* completed in {{duration}}ms",
  "parse_mode": "Markdown"
}`;

interface WebhookSettingsDialogProps {
  isOpen: boolean;
  workflowId: string;
  config: WebhookConfig | undefined;
  incomingConfig: IncomingWebhookConfig | undefined;
  onClose: () => void;
  onSave: (
    config: WebhookConfig | undefined,
    incomingConfig: IncomingWebhookConfig | undefined
  ) => void;
}

/**
 * Webhook Settings Dialog Component
 */
export function WebhookSettingsDialog({
  isOpen,
  workflowId,
  config,
  incomingConfig,
  onClose,
  onSave,
}: WebhookSettingsDialogProps) {
  const modalId = `webhook-settings-${workflowId}`;
  const contentRef = useRef<HTMLDivElement>(null);

  // Tab state
  const [activeTab, setActiveTab] = useState<TabType>('outgoing');

  // ========================
  // Outgoing Webhook State
  // ========================
  const [enabled, setEnabled] = useState(false);
  const [url, setUrl] = useState('');
  const [trigger, setTrigger] = useState<WebhookTrigger>('always');
  const [headers, setHeaders] = useState<Array<{ key: string; value: string }>>([]);
  const [payloadTemplate, setPayloadTemplate] = useState('');
  const [payloadFormat, setPayloadFormat] = useState<PayloadFormat>('custom');
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Outgoing validation state
  const [urlError, setUrlError] = useState<string | null>(null);
  const [jsonError, setJsonError] = useState<string | null>(null);

  // Outgoing test state
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<WebhookTestResult | null>(null);

  // ========================
  // Incoming Webhook State
  // ========================
  const [incomingEnabled, setIncomingEnabled] = useState(false);
  const [incomingToken, setIncomingToken] = useState('');
  const [incomingTokenCreatedAt, setIncomingTokenCreatedAt] = useState('');
  const [incomingPort, setIncomingPort] = useState(DEFAULT_INCOMING_WEBHOOK_PORT);
  const [serverStatus, setServerStatus] = useState<IncomingWebhookServerStatus | null>(null);
  const [isLoadingServerStatus, setIsLoadingServerStatus] = useState(false);
  const [isRegeneratingToken, setIsRegeneratingToken] = useState(false);
  const [copySuccess, setCopySuccess] = useState(false);
  const [portStatus, setPortStatus] = useState<PortStatus | null>(null);
  const [isCheckingPort, setIsCheckingPort] = useState(false);

  // Security settings state
  const [incomingSecret, setIncomingSecret] = useState<string | undefined>(undefined);
  const [requireSignature, setRequireSignature] = useState(false);
  const [rateLimitPerMinute, setRateLimitPerMinute] = useState(60);
  const [isGeneratingSecret, setIsGeneratingSecret] = useState(false);
  const [showSecuritySettings, setShowSecuritySettings] = useState(false);

  // Detect payload format from template
  const detectFormat = (template: string): PayloadFormat => {
    const normalized = template.replace(/\s/g, '');
    if (normalized.includes('"content":')) return 'discord';
    if (normalized.includes('"chat_id":') && normalized.includes('"parse_mode":'))
      return 'telegram';
    if (normalized.includes('"text":')) return 'slack';
    return 'custom';
  };

  // Load incoming webhook server status
  const loadServerStatus = async () => {
    setIsLoadingServerStatus(true);
    try {
      const status = await incomingWebhookAPI.getServerStatus();
      setServerStatus(status);
    } catch (error) {
      console.error('Failed to load incoming webhook status:', error);
    } finally {
      setIsLoadingServerStatus(false);
    }
  };

  // Check port availability (with current workflow excluded)
  const checkPortAvailability = async (port: number) => {
    setIsCheckingPort(true);
    try {
      const status = await incomingWebhookAPI.checkPortAvailable(port, workflowId);
      setPortStatus(status);
    } catch (error) {
      console.error('Failed to check port availability:', error);
      setPortStatus(null);
    } finally {
      setIsCheckingPort(false);
    }
  };

  // Helper to extract workflow name from port status
  const getPortStatusWorkflowName = (status: PortStatus): string | null => {
    if (typeof status === 'object' && 'InUseByWorkflow' in status) {
      return status.InUseByWorkflow;
    }
    return null;
  };

  // Helper to check if port is available
  const isPortAvailable = (status: PortStatus | null): boolean => {
    return status === 'Available';
  };

  // Helper to check if port is used by other workflow
  const isPortUsedByOtherWorkflow = (status: PortStatus | null): boolean => {
    if (status === null) return false;
    return typeof status === 'object' && 'InUseByWorkflow' in status;
  };

  // Helper to check if port is used by external service
  const isPortUsedByOther = (status: PortStatus | null): boolean => {
    return status === 'InUseByOther';
  };

  // Register/unregister modal
  useEffect(() => {
    if (!isOpen) return;
    registerModal(modalId);
    return () => unregisterModal(modalId);
  }, [modalId, isOpen]);

  // Handle ESC key
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      if (!isTopModal(modalId)) return;
      e.preventDefault();
      onClose();
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [modalId, onClose, isOpen]);

  // Focus content area when opened
  useEffect(() => {
    if (isOpen && contentRef.current) {
      const timer = setTimeout(() => {
        contentRef.current?.focus();
      }, 50);
      return () => clearTimeout(timer);
    }
  }, [isOpen]);

  // Check port when it changes
  useEffect(() => {
    if (isOpen && incomingPort >= 1024 && incomingPort <= 65535) {
      const timer = setTimeout(() => {
        checkPortAvailability(incomingPort);
      }, 300); // Debounce
      return () => clearTimeout(timer);
    }
  }, [isOpen, incomingPort, workflowId]);

  // Reset form when dialog opens
  useEffect(() => {
    if (isOpen) {
      // Reset outgoing webhook state
      if (config) {
        setEnabled(config.enabled);
        setUrl(config.url);
        setTrigger(config.trigger);
        setHeaders(
          config.headers
            ? Object.entries(config.headers).map(([key, value]) => ({ key, value }))
            : []
        );
        const template = config.payloadTemplate || '';
        setPayloadTemplate(template);
        setPayloadFormat(template ? detectFormat(template) : 'custom');
        setShowAdvanced(
          !!(config.headers && Object.keys(config.headers).length > 0) || !!config.payloadTemplate
        );
      } else {
        setEnabled(false);
        setUrl('');
        setTrigger('always');
        setHeaders([]);
        setPayloadTemplate('');
        setPayloadFormat('custom');
        setShowAdvanced(false);
      }

      // Reset incoming webhook state from incomingConfig (includes port and security settings)
      if (incomingConfig) {
        setIncomingEnabled(incomingConfig.enabled);
        setIncomingToken(incomingConfig.token);
        setIncomingTokenCreatedAt(incomingConfig.tokenCreatedAt);
        setIncomingPort(incomingConfig.port || DEFAULT_INCOMING_WEBHOOK_PORT);
        setIncomingSecret(incomingConfig.secret);
        setRequireSignature(incomingConfig.requireSignature || false);
        setRateLimitPerMinute(incomingConfig.rateLimitPerMinute || 60);
        setShowSecuritySettings(!!incomingConfig.secret || incomingConfig.requireSignature);
      } else {
        setIncomingEnabled(false);
        setIncomingToken('');
        setIncomingTokenCreatedAt('');
        setIncomingPort(DEFAULT_INCOMING_WEBHOOK_PORT);
        setIncomingSecret(undefined);
        setRequireSignature(false);
        setRateLimitPerMinute(60);
        setShowSecuritySettings(false);
      }

      setTestResult(null);
      setCopySuccess(false);
      setPortStatus(null);

      // Validate existing URL if enabled
      if (config?.enabled && config?.url) {
        // Defer validation to next tick to ensure state is set
        setTimeout(() => {
          try {
            const parsed = new URL(config.url);
            if (parsed.protocol !== 'https:') {
              setUrlError('Only HTTPS URLs are supported');
            } else {
              setUrlError(null);
            }
          } catch {
            setUrlError('Invalid URL format');
          }
        }, 0);
      } else {
        setUrlError(null);
      }
      setJsonError(null);

      // Load server status
      loadServerStatus();
    }
  }, [isOpen, config, incomingConfig]);

  // URL validation
  const validateUrl = (value: string): boolean => {
    if (!value.trim()) {
      setUrlError('URL is required');
      return false;
    }

    try {
      const parsed = new URL(value);
      if (parsed.protocol !== 'https:') {
        setUrlError('Only HTTPS URLs are supported');
        return false;
      }
      setUrlError(null);
      return true;
    } catch {
      setUrlError('Invalid URL format');
      return false;
    }
  };

  const handleUrlChange = (value: string) => {
    setUrl(value);
    if (value) {
      validateUrl(value);
    } else {
      setUrlError(null);
    }
  };

  // JSON validation
  const validateJson = (value: string): boolean => {
    if (!value.trim()) {
      setJsonError(null);
      return true;
    }

    try {
      JSON.parse(value);
      setJsonError(null);
      return true;
    } catch (e) {
      const errorMsg = e instanceof Error ? e.message : 'Invalid JSON';
      setJsonError(errorMsg);
      return false;
    }
  };

  // Handle payload format change
  const handleFormatChange = (format: PayloadFormat) => {
    setPayloadFormat(format);
    if (format === 'discord') {
      setPayloadTemplate(DISCORD_TEMPLATE);
      setJsonError(null);
    } else if (format === 'slack') {
      setPayloadTemplate(SLACK_TEMPLATE);
      setJsonError(null);
    } else if (format === 'telegram') {
      setPayloadTemplate(TELEGRAM_TEMPLATE);
      setJsonError(null);
    }
  };

  // Handle payload template change
  const handlePayloadChange = (value: string) => {
    setPayloadTemplate(value);
    setPayloadFormat('custom');
    validateJson(value);
  };

  // Header management
  const addHeader = () => {
    setHeaders([...headers, { key: '', value: '' }]);
    setShowAdvanced(true);
  };

  const updateHeader = (index: number, field: 'key' | 'value', value: string) => {
    const newHeaders = [...headers];
    newHeaders[index][field] = value;
    setHeaders(newHeaders);
  };

  const removeHeader = (index: number) => {
    setHeaders(headers.filter((_, i) => i !== index));
  };

  // Test webhook
  const handleTest = async () => {
    if (!validateUrl(url)) return;

    setIsTesting(true);
    setTestResult(null);

    try {
      const headersObj = headers
        .filter((h) => h.key.trim())
        .reduce((acc, h) => ({ ...acc, [h.key]: h.value }), {} as Record<string, string>);

      const result = await webhookAPI.testWebhook(
        url,
        Object.keys(headersObj).length > 0 ? headersObj : undefined,
        payloadTemplate || undefined
      );

      setTestResult(result);
    } catch (error) {
      setTestResult({
        success: false,
        error: error instanceof Error ? error.message : 'Test failed',
        responseTime: 0,
      });
    } finally {
      setIsTesting(false);
    }
  };

  // ========================
  // Incoming Webhook Handlers
  // ========================

  // Initialize incoming webhook config (with default port)
  const handleInitIncomingConfig = async () => {
    try {
      const newConfig = await incomingWebhookAPI.createConfig();
      setIncomingToken(newConfig.token);
      setIncomingTokenCreatedAt(newConfig.tokenCreatedAt);
      setIncomingPort(newConfig.port);
      setIncomingSecret(newConfig.secret);
      setRequireSignature(newConfig.requireSignature || false);
      setRateLimitPerMinute(newConfig.rateLimitPerMinute || 60);
      setIncomingEnabled(true);
    } catch (error) {
      console.error('Failed to create incoming webhook config:', error);
    }
  };

  // Generate HMAC secret
  const handleGenerateSecret = async () => {
    setIsGeneratingSecret(true);
    try {
      const secret = await incomingWebhookAPI.generateSecret();
      setIncomingSecret(secret);
    } catch (error) {
      console.error('Failed to generate secret:', error);
    } finally {
      setIsGeneratingSecret(false);
    }
  };

  // Regenerate token
  const handleRegenerateToken = async () => {
    if (!incomingToken) return;

    setIsRegeneratingToken(true);
    try {
      const updatedConfig = await incomingWebhookAPI.regenerateToken({
        enabled: incomingEnabled,
        token: incomingToken,
        tokenCreatedAt: incomingTokenCreatedAt,
        port: incomingPort,
        secret: incomingSecret,
        requireSignature,
        rateLimitPerMinute,
      });
      setIncomingToken(updatedConfig.token);
      setIncomingTokenCreatedAt(updatedConfig.tokenCreatedAt);
    } catch (error) {
      console.error('Failed to regenerate token:', error);
    } finally {
      setIsRegeneratingToken(false);
    }
  };

  // Copy webhook URL or curl command (dynamic based on security settings)
  const handleCopyUrl = async () => {
    // If HMAC secret is configured, copy the full curl command with signature
    const textToCopy = incomingSecret
      ? generateCurlWithSignature(incomingPort, incomingSecret)
      : `curl -X POST "${generateWebhookUrl(incomingPort, incomingToken)}"`;
    try {
      await navigator.clipboard.writeText(textToCopy);
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  };

  // Toggle incoming webhook enabled (only updates local state, saved on Save button)
  const handleToggleIncoming = (newEnabled: boolean) => {
    setIncomingEnabled(newEnabled);
  };

  // Get this workflow's server status
  const getWorkflowServerStatus = () => {
    if (!serverStatus) return null;
    return serverStatus.runningServers.find((s) => s.workflowId === workflowId);
  };

  // Save configuration
  const handleSave = async () => {
    if (enabled && !validateUrl(url)) return;
    if (enabled && payloadTemplate.trim() && !validateJson(payloadTemplate)) return;

    // Build outgoing config - save even when disabled if there's any configuration
    let outgoingConfig: WebhookConfig | undefined;
    const hasOutgoingConfig = url.trim() || headers.length > 0 || payloadTemplate.trim();
    if (enabled || hasOutgoingConfig) {
      const headersObj = headers
        .filter((h) => h.key.trim())
        .reduce((acc, h) => ({ ...acc, [h.key]: h.value }), {} as Record<string, string>);

      outgoingConfig = {
        enabled,
        url,
        trigger,
        headers: Object.keys(headersObj).length > 0 ? headersObj : undefined,
        payloadTemplate: payloadTemplate.trim() || undefined,
      };
    }

    // Build incoming config (now includes port and security settings)
    let newIncomingConfig: IncomingWebhookConfig | undefined;
    if (incomingToken) {
      newIncomingConfig = {
        enabled: incomingEnabled,
        token: incomingToken,
        tokenCreatedAt: incomingTokenCreatedAt,
        port: incomingPort,
        secret: incomingSecret,
        requireSignature,
        rateLimitPerMinute,
      };
    }

    // Save workflow (this triggers server sync which will start/stop/restart servers as needed)
    onSave(outgoingConfig, newIncomingConfig);
    onClose();
  };

  // Handle backdrop click
  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      onClose();
    }
  };

  // ========================
  // Render
  // ========================

  const workflowServerStatus = getWorkflowServerStatus();

  if (!isOpen) return null;

  return (
    <div
      className={cn('fixed inset-0 z-50', 'animate-in fade-in-0 duration-200')}
      role="dialog"
      aria-modal="true"
      aria-labelledby="webhook-settings-dialog-title"
    >
      {/* Backdrop */}
      <div
        className="fixed inset-0 bg-black/70 backdrop-blur-sm"
        onClick={handleBackdropClick}
        aria-hidden="true"
      />

      {/* Dialog container */}
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <div
          className={cn(
            'relative w-full max-w-xl max-h-[90vh]',
            'bg-background rounded-2xl',
            'border border-purple-500/30',
            'shadow-2xl shadow-black/60',
            'animate-in fade-in-0 zoom-in-95 duration-200',
            'slide-in-from-bottom-4',
            'flex flex-col overflow-hidden'
          )}
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header with gradient */}
          <div
            className={cn(
              'relative px-6 py-5',
              'border-b border-border',
              'bg-gradient-to-r',
              'from-purple-500/10 via-purple-600/5 to-transparent',
              'dark:from-purple-500/20 dark:via-purple-600/10 dark:to-transparent'
            )}
          >
            {/* Close button */}
            <Button
              variant="ghost"
              size="icon"
              onClick={onClose}
              className="absolute right-4 top-4"
              aria-label="Close dialog"
            >
              <X className="w-4 h-4" />
            </Button>

            {/* Title area with icon badge */}
            <div className="flex items-start gap-4 pr-10">
              <div
                className={cn(
                  'flex-shrink-0',
                  'w-12 h-12 rounded-xl',
                  'flex items-center justify-center',
                  'bg-background/80 dark:bg-background/50 backdrop-blur-sm',
                  'border',
                  'bg-purple-500/10 border-purple-500/20',
                  'shadow-lg'
                )}
              >
                <Webhook className="w-6 h-6 text-purple-500 dark:text-purple-400" />
              </div>
              <div className="flex-1 min-w-0 pt-1">
                <h2
                  id="webhook-settings-dialog-title"
                  className="text-lg font-semibold text-foreground leading-tight"
                >
                  Webhook Settings
                </h2>
                <p className="mt-1 text-sm text-muted-foreground">
                  Configure outgoing notifications and incoming triggers
                </p>
              </div>
            </div>
          </div>

          {/* Tab Navigation */}
          <div className="flex border-b border-border bg-card/30">
            <button
              type="button"
              className={cn(
                'flex-1 px-4 py-3 text-sm font-medium',
                'flex items-center justify-center gap-2',
                'transition-all duration-150',
                'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-inset',
                activeTab === 'outgoing'
                  ? 'text-purple-600 dark:text-purple-400 border-b-2 border-purple-500 bg-purple-500/5'
                  : 'text-muted-foreground hover:text-foreground hover:bg-accent/30'
              )}
              onClick={() => setActiveTab('outgoing')}
            >
              <ArrowUpFromLine className="w-4 h-4" />
              Outgoing
            </button>
            <button
              type="button"
              className={cn(
                'flex-1 px-4 py-3 text-sm font-medium',
                'flex items-center justify-center gap-2',
                'transition-all duration-150',
                'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-inset',
                activeTab === 'incoming'
                  ? 'text-purple-600 dark:text-purple-400 border-b-2 border-purple-500 bg-purple-500/5'
                  : 'text-muted-foreground hover:text-foreground hover:bg-accent/30'
              )}
              onClick={() => setActiveTab('incoming')}
            >
              <ArrowDownToLine className="w-4 h-4" />
              Incoming
            </button>
          </div>

          {/* Scrollable body */}
          <div
            ref={contentRef}
            className="flex-1 overflow-y-auto min-h-0 p-6 focus:outline-none"
            tabIndex={-1}
          >
            {activeTab === 'outgoing' ? (
              // ========================
              // Outgoing Webhook Tab
              // ========================
              <div className="space-y-5">
                {/* Usage hint - only show when disabled */}
                {!enabled && (
                  <div
                    className={cn(
                      'p-4 rounded-xl',
                      'bg-purple-500/5 dark:bg-purple-500/10',
                      'border border-purple-500/20',
                      'text-xs'
                    )}
                  >
                    <p className="font-medium mb-1.5 text-purple-700 dark:text-purple-300">
                      Outgoing Webhook
                    </p>
                    <p className="text-purple-600 dark:text-purple-400">
                      Send a notification when this workflow completes. Use cases:
                    </p>
                    <ul className="mt-1.5 ml-4 list-disc text-purple-600/80 dark:text-purple-400/80 space-y-0.5">
                      <li>Notify Slack or Discord channel</li>
                      <li>Trigger downstream CI/CD pipelines</li>
                      <li>Log results to monitoring systems</li>
                    </ul>
                  </div>
                )}

                {/* Enable/Disable Toggle */}
                <div
                  className={cn(
                    'flex items-center justify-between p-4',
                    'bg-muted/30 dark:bg-muted/50 rounded-xl',
                    'border border-border/50',
                    'transition-colors duration-150'
                  )}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className={cn(
                        'w-8 h-8 rounded-lg',
                        'flex items-center justify-center',
                        'bg-background border border-border'
                      )}
                    >
                      <Bell className="w-4 h-4 text-muted-foreground" />
                    </div>
                    <span className="text-sm font-medium text-foreground">Enable Webhook</span>
                  </div>
                  <Toggle
                    checked={enabled}
                    onChange={setEnabled}
                    size="lg"
                    aria-label="Enable outgoing webhook"
                  />
                </div>

                {/* URL Input */}
                <div className="space-y-2">
                  <label className="flex items-center gap-2 text-sm font-medium text-foreground">
                    <Link className="w-4 h-4 text-muted-foreground" />
                    Webhook URL
                  </label>
                  <Input
                    value={url}
                    onChange={(e) => handleUrlChange(e.target.value)}
                    placeholder="https://example.com/webhook"
                    disabled={!enabled}
                    className={cn(
                      'bg-background border-border text-foreground',
                      'focus:border-purple-500 focus:ring-purple-500/20',
                      'transition-all duration-150',
                      urlError && 'border-red-500 focus:border-red-500 focus:ring-red-500/20',
                      !enabled && 'opacity-60'
                    )}
                  />
                  {urlError && (
                    <p className="text-xs text-red-500 dark:text-red-400 flex items-center gap-1.5">
                      <AlertCircle className="w-3 h-3" />
                      {urlError}
                    </p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    Only HTTPS URLs are supported for security.
                  </p>
                </div>

                {/* Trigger Condition */}
                <div className="space-y-2">
                  <label className="text-sm font-medium text-foreground">Trigger Condition</label>
                  <div className="grid grid-cols-3 gap-2">
                    {[
                      { value: 'always', label: 'Always' },
                      { value: 'onSuccess', label: 'On Success' },
                      { value: 'onFailure', label: 'On Failure' },
                    ].map((option) => (
                      <button
                        key={option.value}
                        type="button"
                        disabled={!enabled}
                        className={cn(
                          'px-3 py-2.5 text-sm rounded-lg border',
                          'transition-all duration-150',
                          'focus:outline-none focus:ring-2 focus:ring-ring',
                          trigger === option.value
                            ? 'bg-purple-500/10 border-purple-500 text-purple-700 dark:text-purple-300 font-medium'
                            : 'bg-background border-border text-muted-foreground hover:border-purple-500/50 hover:text-foreground',
                          !enabled && 'opacity-50 cursor-not-allowed'
                        )}
                        onClick={() => setTrigger(option.value as WebhookTrigger)}
                      >
                        {option.label}
                      </button>
                    ))}
                  </div>
                </div>

                {/* Advanced Settings Toggle */}
                <button
                  type="button"
                  className={cn(
                    'text-sm font-medium',
                    'text-purple-600 dark:text-purple-400',
                    'hover:text-purple-700 dark:hover:text-purple-300',
                    'flex items-center gap-2',
                    'transition-colors duration-150',
                    'focus:outline-none focus:underline'
                  )}
                  onClick={() => setShowAdvanced(!showAdvanced)}
                >
                  <span
                    className={cn(
                      'w-4 h-4 flex items-center justify-center',
                      'transition-transform duration-200',
                      showAdvanced && 'rotate-90'
                    )}
                  >
                    {'>'}
                  </span>
                  Advanced Settings
                </button>

                {showAdvanced && (
                  <div className="space-y-5 pl-4 border-l-2 border-purple-500/20">
                    {/* Custom Headers */}
                    <div className="space-y-3">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium text-foreground">
                          Custom Headers
                        </label>
                        <Button
                          variant="ghost"
                          size="sm"
                          disabled={!enabled}
                          onClick={addHeader}
                          className={cn(
                            'text-purple-600 dark:text-purple-400',
                            'hover:text-purple-700 dark:hover:text-purple-300',
                            'hover:bg-purple-500/10',
                            'text-xs'
                          )}
                        >
                          <Plus className="w-3 h-3 mr-1" />
                          Add Header
                        </Button>
                      </div>
                      {headers.map((header, index) => (
                        <div key={index} className="flex gap-2">
                          <Input
                            value={header.key}
                            onChange={(e) => updateHeader(index, 'key', e.target.value)}
                            placeholder="Header name"
                            disabled={!enabled}
                            className={cn(
                              'bg-background border-border text-foreground text-sm flex-1',
                              'focus:border-purple-500 focus:ring-purple-500/20',
                              !enabled && 'opacity-60'
                            )}
                          />
                          <Input
                            value={header.value}
                            onChange={(e) => updateHeader(index, 'value', e.target.value)}
                            placeholder="Value"
                            disabled={!enabled}
                            className={cn(
                              'bg-background border-border text-foreground text-sm flex-1',
                              'focus:border-purple-500 focus:ring-purple-500/20',
                              !enabled && 'opacity-60'
                            )}
                          />
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={!enabled}
                            onClick={() => removeHeader(index)}
                            className="text-red-500 hover:text-red-400 hover:bg-red-500/10 px-2"
                          >
                            <Trash2 className="w-4 h-4" />
                          </Button>
                        </div>
                      ))}
                      {headers.length === 0 && (
                        <p className="text-xs text-muted-foreground">
                          No custom headers configured.
                        </p>
                      )}
                    </div>

                    {/* Payload Template */}
                    <div className="space-y-3">
                      <label className="flex items-center gap-2 text-sm font-medium text-foreground">
                        <FileCode className="w-4 h-4 text-muted-foreground" />
                        Payload Template (JSON)
                      </label>

                      {/* Format Presets */}
                      <div className="flex flex-wrap gap-2">
                        {[
                          { value: 'custom', label: 'Custom' },
                          { value: 'discord', label: 'Discord' },
                          { value: 'slack', label: 'Slack' },
                          { value: 'telegram', label: 'Telegram' },
                        ].map((option) => (
                          <button
                            key={option.value}
                            type="button"
                            disabled={!enabled}
                            className={cn(
                              'px-3 py-1.5 text-xs rounded-lg border',
                              'transition-all duration-150',
                              'focus:outline-none focus:ring-2 focus:ring-ring',
                              payloadFormat === option.value
                                ? 'bg-purple-500/10 border-purple-500 text-purple-700 dark:text-purple-300 font-medium'
                                : 'bg-background border-border text-muted-foreground hover:border-purple-500/50 hover:text-foreground',
                              !enabled && 'opacity-50 cursor-not-allowed'
                            )}
                            onClick={() => handleFormatChange(option.value as PayloadFormat)}
                          >
                            {option.label}
                          </button>
                        ))}
                      </div>

                      <textarea
                        value={payloadTemplate}
                        onChange={(e) => handlePayloadChange(e.target.value)}
                        placeholder={DEFAULT_PAYLOAD_TEMPLATE}
                        disabled={!enabled}
                        rows={6}
                        autoComplete="off"
                        autoCorrect="off"
                        autoCapitalize="off"
                        spellCheck={false}
                        className={cn(
                          'w-full bg-background border rounded-lg p-3',
                          'text-foreground font-mono text-xs resize-none',
                          'transition-all duration-150',
                          'focus:outline-none focus:ring-2',
                          jsonError
                            ? 'border-red-500 focus:border-red-500 focus:ring-red-500/20'
                            : 'border-border focus:border-purple-500 focus:ring-purple-500/20',
                          !enabled && 'opacity-60'
                        )}
                      />
                      {jsonError && (
                        <p className="text-xs text-red-500 dark:text-red-400 flex items-center gap-1.5">
                          <AlertCircle className="w-3 h-3" />
                          Invalid JSON: {jsonError}
                        </p>
                      )}
                      <div className="text-xs text-muted-foreground">
                        <p className="mb-1.5">Available variables:</p>
                        <div className="flex flex-wrap gap-1.5">
                          {SUPPORTED_VARIABLES.map((v) => (
                            <code
                              key={v}
                              className={cn(
                                'px-1.5 py-0.5 rounded',
                                'bg-muted/50 text-foreground',
                                'border border-border/50'
                              )}
                            >
                              {`{{${v}}}`}
                            </code>
                          ))}
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {/* Test Result */}
                {testResult && (
                  <div
                    className={cn(
                      'p-4 rounded-xl border',
                      testResult.success
                        ? 'bg-green-500/5 dark:bg-green-500/10 border-green-500/30'
                        : 'bg-red-500/5 dark:bg-red-500/10 border-red-500/30'
                    )}
                  >
                    <div className="flex items-center gap-2 mb-2">
                      {testResult.success ? (
                        <CheckCircle className="w-4 h-4 text-green-500" />
                      ) : (
                        <AlertCircle className="w-4 h-4 text-red-500" />
                      )}
                      <span
                        className={cn(
                          'text-sm font-medium',
                          testResult.success
                            ? 'text-green-600 dark:text-green-400'
                            : 'text-red-600 dark:text-red-400'
                        )}
                      >
                        {testResult.success ? 'Test Successful' : 'Test Failed'}
                      </span>
                    </div>
                    <div className="text-xs text-muted-foreground space-y-1">
                      {testResult.statusCode && <p>Status: {testResult.statusCode}</p>}
                      <p>Response time: {testResult.responseTime}ms</p>
                      {testResult.error && (
                        <p className="text-red-500 dark:text-red-400">Error: {testResult.error}</p>
                      )}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              // ========================
              // Incoming Webhook Tab
              // ========================
              <div className="space-y-5">
                {/* Usage hint */}
                {!incomingEnabled && !incomingToken && (
                  <div
                    className={cn(
                      'p-4 rounded-xl',
                      'bg-purple-500/5 dark:bg-purple-500/10',
                      'border border-purple-500/20',
                      'text-xs'
                    )}
                  >
                    <p className="font-medium mb-1.5 text-purple-700 dark:text-purple-300">
                      Incoming Webhook
                    </p>
                    <p className="text-purple-600 dark:text-purple-400">
                      Allow external systems to trigger this workflow via HTTP request. Use cases:
                    </p>
                    <ul className="mt-1.5 ml-4 list-disc text-purple-600/80 dark:text-purple-400/80 space-y-0.5">
                      <li>Trigger from CI/CD pipeline</li>
                      <li>Integrate with external automation tools</li>
                      <li>Start workflow via cron job or scheduler</li>
                    </ul>
                  </div>
                )}

                {/* Server Status for this workflow */}
                <div
                  className={cn(
                    'p-4 rounded-xl',
                    'bg-muted/30 dark:bg-muted/50',
                    'border border-border/50'
                  )}
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-sm font-medium text-foreground">Server Status</span>
                    {isLoadingServerStatus ? (
                      <Loader2 className="w-4 h-4 text-muted-foreground animate-spin" />
                    ) : workflowServerStatus?.running ? (
                      <span className="flex items-center gap-1.5 text-xs text-green-600 dark:text-green-400">
                        <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
                        Running on port {workflowServerStatus.port}
                      </span>
                    ) : (
                      <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                        <span className="w-2 h-2 bg-muted-foreground/50 rounded-full"></span>
                        {incomingEnabled ? 'Will start on save' : 'Not running'}
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    Each workflow has its own dedicated server on the configured port.
                  </p>
                </div>

                {/* Port Configuration */}
                <div className="space-y-2">
                  <label className="text-sm font-medium text-foreground">Server Port</label>
                  <div className="flex items-center gap-2">
                    <Input
                      type="number"
                      value={incomingPort}
                      onChange={(e) =>
                        setIncomingPort(parseInt(e.target.value) || DEFAULT_INCOMING_WEBHOOK_PORT)
                      }
                      min={1024}
                      max={65535}
                      className={cn(
                        'bg-background border-border text-foreground flex-1',
                        'focus:border-purple-500 focus:ring-purple-500/20',
                        isPortUsedByOther(portStatus) &&
                          'border-red-500 focus:border-red-500 focus:ring-red-500/20'
                      )}
                    />
                    {isCheckingPort && (
                      <Loader2 className="w-4 h-4 text-muted-foreground animate-spin" />
                    )}
                    {!isCheckingPort && isPortAvailable(portStatus) && (
                      <CheckCircle className="w-4 h-4 text-green-500" />
                    )}
                    {!isCheckingPort && isPortUsedByOtherWorkflow(portStatus) && (
                      <AlertCircle className="w-4 h-4 text-yellow-500" />
                    )}
                    {!isCheckingPort && isPortUsedByOther(portStatus) && (
                      <AlertCircle className="w-4 h-4 text-red-500" />
                    )}
                  </div>
                  <p
                    className={cn(
                      'text-xs',
                      isPortUsedByOther(portStatus)
                        ? 'text-red-500 dark:text-red-400'
                        : isPortUsedByOtherWorkflow(portStatus)
                          ? 'text-yellow-600 dark:text-yellow-400'
                          : 'text-muted-foreground'
                    )}
                  >
                    {isPortUsedByOther(portStatus)
                      ? 'This port is in use by another service. Choose a different port.'
                      : isPortUsedByOtherWorkflow(portStatus)
                        ? `This port is used by workflow "${getPortStatusWorkflowName(portStatus!)}". Choose a different port.`
                        : "Port number for this workflow's webhook server (1024-65535)."}
                  </p>
                </div>

                {/* Enable/Disable Toggle */}
                <div
                  className={cn(
                    'flex items-center justify-between p-4',
                    'bg-muted/30 dark:bg-muted/50 rounded-xl',
                    'border border-border/50',
                    'transition-colors duration-150'
                  )}
                >
                  <div className="flex items-center gap-3">
                    <div
                      className={cn(
                        'w-8 h-8 rounded-lg',
                        'flex items-center justify-center',
                        'bg-background border border-border'
                      )}
                    >
                      <Bell className="w-4 h-4 text-muted-foreground" />
                    </div>
                    <span className="text-sm font-medium text-foreground">
                      Enable Incoming Webhook
                    </span>
                  </div>
                  <Toggle
                    checked={incomingEnabled}
                    onChange={(checked) => {
                      if (!incomingToken) {
                        handleInitIncomingConfig();
                      } else {
                        handleToggleIncoming(checked);
                      }
                    }}
                    size="lg"
                    aria-label="Enable incoming webhook"
                  />
                </div>

                {/* Token & URL Section */}
                {incomingToken && (
                  <div className="space-y-5">
                    {/* API Token */}
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <label className="text-sm font-medium text-foreground">API Token</label>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={handleRegenerateToken}
                          disabled={isRegeneratingToken}
                          className={cn(
                            'text-purple-600 dark:text-purple-400',
                            'hover:text-purple-700 dark:hover:text-purple-300',
                            'hover:bg-purple-500/10',
                            'text-xs'
                          )}
                        >
                          {isRegeneratingToken ? (
                            <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                          ) : (
                            <RefreshCw className="w-3 h-3 mr-1" />
                          )}
                          Regenerate
                        </Button>
                      </div>
                      <Input
                        value={incomingToken}
                        readOnly
                        className="bg-muted/30 border-border text-muted-foreground font-mono text-xs"
                      />
                      {incomingTokenCreatedAt && (
                        <p className="text-xs text-muted-foreground">
                          Created: {new Date(incomingTokenCreatedAt).toLocaleString()}
                        </p>
                      )}
                    </div>

                    {/* Webhook URL */}
                    <div className="space-y-2">
                      <label className="text-sm font-medium text-foreground">
                        Webhook {incomingSecret ? 'Endpoint' : 'URL'}
                      </label>
                      <div className="flex gap-2">
                        <Input
                          value={
                            incomingSecret
                              ? `http://localhost:${incomingPort}/webhook`
                              : generateWebhookUrl(incomingPort, incomingToken)
                          }
                          readOnly
                          className="bg-muted/30 border-border text-muted-foreground font-mono text-xs flex-1"
                        />
                        <Button
                          variant="outline"
                          onClick={handleCopyUrl}
                          title={
                            incomingSecret
                              ? 'Copy curl command with signature'
                              : 'Copy curl command'
                          }
                          className={cn(
                            'px-3 border-border',
                            'transition-all duration-150',
                            copySuccess
                              ? 'text-green-500 border-green-500 bg-green-500/10'
                              : 'text-foreground hover:bg-accent hover:border-purple-500/50'
                          )}
                        >
                          {copySuccess ? (
                            <CheckCircle className="w-4 h-4" />
                          ) : (
                            <Copy className="w-4 h-4" />
                          )}
                        </Button>
                      </div>
                      <p className="text-xs text-muted-foreground">
                        {incomingSecret
                          ? 'HMAC signature required. Copy button copies full curl command.'
                          : 'POST request to this URL will trigger the workflow.'}
                      </p>
                    </div>

                    {/* Security Settings Toggle */}
                    <button
                      type="button"
                      className={cn(
                        'text-sm font-medium',
                        'text-purple-600 dark:text-purple-400',
                        'hover:text-purple-700 dark:hover:text-purple-300',
                        'flex items-center gap-2',
                        'transition-colors duration-150',
                        'focus:outline-none focus:underline'
                      )}
                      onClick={() => setShowSecuritySettings(!showSecuritySettings)}
                    >
                      <span
                        className={cn(
                          'w-4 h-4 flex items-center justify-center',
                          'transition-transform duration-200',
                          showSecuritySettings && 'rotate-90'
                        )}
                      >
                        {'>'}
                      </span>
                      Security Settings
                    </button>

                    {showSecuritySettings && (
                      <div className="space-y-5 pl-4 border-l-2 border-purple-500/20">
                        {/* HMAC Secret */}
                        <div className="space-y-2">
                          <div className="flex items-center justify-between">
                            <label className="text-sm font-medium text-foreground">
                              HMAC Secret (Optional)
                            </label>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={handleGenerateSecret}
                              disabled={isGeneratingSecret}
                              className={cn(
                                'text-purple-600 dark:text-purple-400',
                                'hover:text-purple-700 dark:hover:text-purple-300',
                                'hover:bg-purple-500/10',
                                'text-xs'
                              )}
                            >
                              {isGeneratingSecret ? (
                                <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                              ) : (
                                <RefreshCw className="w-3 h-3 mr-1" />
                              )}
                              Generate
                            </Button>
                          </div>
                          <Input
                            value={incomingSecret || ''}
                            onChange={(e) => setIncomingSecret(e.target.value || undefined)}
                            placeholder="Leave empty to use token authentication"
                            className="bg-background border-border text-foreground font-mono text-xs"
                          />
                          <p className="text-xs text-muted-foreground">
                            When set, requests must include X-Webhook-Signature header with
                            HMAC-SHA256 signature.
                          </p>
                        </div>

                        {/* Require Signature Toggle */}
                        <div
                          className={cn(
                            'flex items-center justify-between p-3',
                            'bg-muted/20 rounded-lg',
                            'border border-border/50'
                          )}
                        >
                          <div>
                            <span className="text-sm font-medium text-foreground">
                              Require Signature
                            </span>
                            <p className="text-xs text-muted-foreground mt-0.5">
                              Reject requests without valid signature
                            </p>
                          </div>
                          <Toggle
                            checked={requireSignature}
                            onChange={setRequireSignature}
                            disabled={!incomingSecret}
                            size="lg"
                            aria-label="Require signature"
                          />
                        </div>

                        {/* Rate Limit */}
                        <div className="space-y-2">
                          <label className="text-sm font-medium text-foreground">
                            Rate Limit (requests/minute)
                          </label>
                          <Input
                            type="number"
                            value={rateLimitPerMinute}
                            onChange={(e) =>
                              setRateLimitPerMinute(Math.max(1, parseInt(e.target.value) || 60))
                            }
                            min={1}
                            max={1000}
                            className="bg-background border-border text-foreground"
                          />
                          <p className="text-xs text-muted-foreground">
                            Maximum requests allowed per minute per IP address. Default: 60.
                          </p>
                        </div>
                      </div>
                    )}

                    {/* Usage Example */}
                    <div className="space-y-2">
                      <label className="text-sm font-medium text-foreground">Usage Example</label>
                      <div
                        className={cn(
                          'bg-muted/30 border border-border rounded-lg p-3',
                          'overflow-x-auto'
                        )}
                      >
                        <code className="text-xs text-foreground font-mono whitespace-pre-wrap break-all">
                          {incomingSecret
                            ? generateCurlWithSignature(incomingPort, incomingSecret)
                            : `curl -X POST "${generateWebhookUrl(incomingPort, incomingToken)}"`}
                        </code>
                      </div>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>

          {/* Footer with actions */}
          <div
            className={cn(
              'px-6 py-4',
              'border-t border-border',
              'bg-card/50',
              'flex items-center justify-between gap-4',
              'flex-shrink-0'
            )}
          >
            {/* Left side - Test button for outgoing tab */}
            {activeTab === 'outgoing' ? (
              <Button
                variant="outline"
                onClick={handleTest}
                disabled={!enabled || !url || !!urlError || !!jsonError || isTesting}
                className={cn(
                  'border-purple-500/50 text-purple-600 dark:text-purple-400',
                  'hover:bg-purple-500/10 hover:border-purple-500',
                  'disabled:opacity-50 disabled:cursor-not-allowed',
                  'transition-all duration-150'
                )}
              >
                {isTesting ? (
                  <Loader2 className="w-4 h-4 mr-1.5 animate-spin" />
                ) : (
                  <Play className="w-4 h-4 mr-1.5" />
                )}
                Test Webhook
              </Button>
            ) : (
              <div /> // Empty div for spacing
            )}

            {/* Right side - Action buttons */}
            <div className="flex items-center gap-2">
              <Button variant="ghost" onClick={onClose}>
                Cancel
              </Button>
              <Button
                onClick={handleSave}
                disabled={
                  // Outgoing validation: if enabled, must have valid URL and JSON
                  enabled && (!url || !!urlError || !!jsonError)
                  // Note: Port conflicts for incoming webhook are just warnings, not blockers
                  // The server will fail to start but the config can still be saved
                }
                variant="success"
              >
                <Webhook className="w-4 h-4 mr-1.5" />
                Save Settings
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
