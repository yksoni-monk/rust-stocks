import { createSignal } from 'solid-js';
import { dataRefreshAPI } from '../services/api';
import type {
  SystemFreshnessReport,
  RefreshProgress,
  RefreshRequestDto,
  RefreshResult,
  RefreshDurationEstimates
} from '../utils/types';

// Data refresh store for managing system data freshness and refresh operations
export function createDataRefreshStore() {
  // State signals
  const [freshnessStatus, setFreshnessStatus] = createSignal<SystemFreshnessReport | null>(null);
  const [refreshProgress, setRefreshProgress] = createSignal<RefreshProgress | null>(null);
  const [lastRefreshResult, setLastRefreshResult] = createSignal<RefreshResult | null>(null);
  const [durationEstimates, setDurationEstimates] = createSignal<RefreshDurationEstimates | null>(null);
  const [isRefreshing, setIsRefreshing] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [currentSessionId, setCurrentSessionId] = createSignal<string | null>(null);

  // Auto-refresh progress polling
  let progressInterval: NodeJS.Timeout | null = null;

  // Check current data freshness status
  const checkDataFreshness = async () => {
    try {
      setError(null);
      const status = await dataRefreshAPI.getDataFreshnessStatus();
      setFreshnessStatus(status);
      console.log('🔍 Data freshness status updated:', status.overall_status);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to check data freshness';
      setError(errorMessage);
      console.error('❌ Failed to check data freshness:', err);
    }
  };

  // Check if specific screening feature is ready
  const checkScreeningReadiness = async (feature: string): Promise<boolean> => {
    try {
      setError(null);
      const isReady = await dataRefreshAPI.checkScreeningReadiness(feature);
      console.log(`🎯 ${feature} screening readiness:`, isReady ? 'Ready' : 'Not ready');
      return isReady;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to check screening readiness';
      setError(errorMessage);
      console.error('❌ Failed to check screening readiness:', err);
      return false;
    }
  };

  // Start data refresh operation
  const startRefresh = async (mode: 'market' | 'financials' | 'ratios', forceRefresh = false) => {
    try {
      setError(null);
      setIsRefreshing(true);

      const request: RefreshRequestDto = {
        mode,
        force_sources: forceRefresh ? [mode] : undefined,
        initiated_by: 'frontend_user'
      };

      console.log(`🔄 Starting ${mode} data refresh...`);
      const sessionId = await dataRefreshAPI.startDataRefresh(request);
      setCurrentSessionId(sessionId);

      // Start polling for progress
      startProgressPolling(sessionId);

      console.log(`✅ ${mode} refresh started with session: ${sessionId}`);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to start refresh';
      setError(errorMessage);
      setIsRefreshing(false);
      console.error('❌ Failed to start refresh:', err);
    }
  };

  // Start bulk refresh operations
  const startBulkRefresh = async (type: 'quick' | 'full') => {
    if (type === 'quick') {
      // Quick refresh: Market + Ratios (~25 min)
      await startRefresh('market');
      // Note: The backend will handle sequencing automatically
    } else {
      // Full refresh: Market + Financials + Ratios (~115 min)
      await startRefresh('market');
      // Note: The backend will handle sequencing automatically
    }
  };

  // Poll for refresh progress
  const startProgressPolling = (sessionId: string) => {
    if (progressInterval) {
      clearInterval(progressInterval);
    }

    progressInterval = setInterval(async () => {
      try {
        const progress = await dataRefreshAPI.getRefreshProgress(sessionId);

        if (progress) {
          setRefreshProgress(progress);

          // Check if refresh is complete
          if (progress.status === 'completed' || progress.status === 'failed') {
            stopProgressPolling();
            setIsRefreshing(false);

            // Update freshness status after completion
            await checkDataFreshness();

            // Get final result
            const result = await dataRefreshAPI.getLastRefreshResult();
            setLastRefreshResult(result);

            console.log(`🎯 Refresh ${progress.status}:`, progress.operation_type);
          }
        }
      } catch (err) {
        console.error('⚠️ Error polling refresh progress:', err);
      }
    }, 2000); // Poll every 2 seconds
  };

  // Stop progress polling
  const stopProgressPolling = () => {
    if (progressInterval) {
      clearInterval(progressInterval);
      progressInterval = null;
    }
  };

  // Cancel ongoing refresh operation
  const cancelRefresh = async () => {
    const sessionId = currentSessionId();
    if (!sessionId) return false;

    try {
      const cancelled = await dataRefreshAPI.cancelRefreshOperation(sessionId);
      if (cancelled) {
        stopProgressPolling();
        setIsRefreshing(false);
        setRefreshProgress(null);
        setCurrentSessionId(null);
        console.log('🚫 Refresh operation cancelled');
      }
      return cancelled;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to cancel refresh';
      setError(errorMessage);
      console.error('❌ Failed to cancel refresh:', err);
      return false;
    }
  };

  // Get refresh duration estimates
  const loadDurationEstimates = async () => {
    try {
      const estimates = await dataRefreshAPI.getRefreshDurationEstimates();
      setDurationEstimates(estimates);
    } catch (err) {
      console.error('⚠️ Failed to load duration estimates:', err);
    }
  };

  // Get last refresh result
  const loadLastRefreshResult = async () => {
    try {
      const result = await dataRefreshAPI.getLastRefreshResult();
      setLastRefreshResult(result);
    } catch (err) {
      console.error('⚠️ Failed to load last refresh result:', err);
    }
  };

  // Auto-refresh setup
  const setupAutoRefresh = () => {
    // Check data freshness every hour
    const checkInterval = setInterval(async () => {
      await checkDataFreshness();

      const status = freshnessStatus();
      if (status?.market_data && !status.market_data.is_fresh) {
        // Auto-refresh market data if stale (low-cost operation)
        console.log('🔄 Auto-refreshing stale market data...');
        await startRefresh('market');
      }
    }, 60 * 60 * 1000); // 1 hour

    // Cleanup interval on unmount
    return () => {
      clearInterval(checkInterval);
      stopProgressPolling();
    };
  };

  // Initialize store
  const initialize = async () => {
    await checkDataFreshness();
    await loadDurationEstimates();
    await loadLastRefreshResult();
    return setupAutoRefresh();
  };

  // Utility functions
  const getDataTypeIcon = (type: 'market' | 'financials' | 'ratios'): string => {
    switch (type) {
      case 'market': return '📈';
      case 'financials': return '📋';
      case 'ratios': return '🧮';
      default: return '📊';
    }
  };

  const getStatusColor = (status: 'fresh' | 'stale' | 'critical'): string => {
    switch (status) {
      case 'fresh': return 'text-green-600';
      case 'stale': return 'text-yellow-600';
      case 'critical': return 'text-red-600';
      default: return 'text-gray-600';
    }
  };

  const getStatusText = (status: 'fresh' | 'stale' | 'critical'): string => {
    switch (status) {
      case 'fresh': return 'Fresh';
      case 'stale': return 'Stale';
      case 'critical': return 'Critical';
      default: return 'Unknown';
    }
  };

  const formatDuration = (minutes: number): string => {
    if (minutes < 60) {
      return `~${minutes} min`;
    } else {
      const hours = Math.floor(minutes / 60);
      const remainingMinutes = minutes % 60;
      return remainingMinutes > 0 ? `~${hours}h ${remainingMinutes}m` : `~${hours}h`;
    }
  };

  return {
    // State
    freshnessStatus,
    refreshProgress,
    lastRefreshResult,
    durationEstimates,
    isRefreshing,
    error,
    currentSessionId,

    // Actions
    checkDataFreshness,
    checkScreeningReadiness,
    startRefresh,
    startBulkRefresh,
    cancelRefresh,
    loadDurationEstimates,
    loadLastRefreshResult,
    initialize,

    // Utilities
    getDataTypeIcon,
    getStatusColor,
    getStatusText,
    formatDuration,

    // Cleanup
    cleanup: stopProgressPolling
  };
}

// Create global store instance
export const dataRefreshStore = createDataRefreshStore();

// Initialize store on import
dataRefreshStore.initialize();