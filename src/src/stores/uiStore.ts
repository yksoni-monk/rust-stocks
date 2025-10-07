import { createSignal } from 'solid-js';

// UI store for managing application UI state
export function createUiStore() {
  // Navigation state
  const [activeTab, setActiveTab] = createSignal<'screening'>('screening');

  // Panel visibility
  const [showRecommendations, setShowRecommendations] = createSignal(false);
  const [showDataFetching, setShowDataFetching] = createSignal(false);
  const [showDataManagement, setShowDataManagement] = createSignal(false);
  
  // Modal state
  const [activeModal, setActiveModal] = createSignal<string | null>(null);
  
  // Global loading states
  const [globalLoading, setGlobalLoading] = createSignal(false);
  const [globalError, setGlobalError] = createSignal<string | null>(null);
  
  // Toast notifications
  const [toasts, setToasts] = createSignal<Array<{
    id: string;
    message: string;
    type: 'success' | 'error' | 'info' | 'warning';
    duration?: number;
  }>>([]);

  // Show recommendations panel
  const openRecommendations = () => {
    setShowRecommendations(true);
    setShowDataFetching(false);
    setShowDataManagement(false);
  };

  // Show data fetching panel
  const openDataFetching = () => {
    setShowDataFetching(true);
    setShowRecommendations(false);
    setShowDataManagement(false);
  };


  // Navigate to screening tab
  const openScreening = () => {
    setActiveTab('screening');
    setShowDataManagement(false);
  };

  // Close all panels
  const closeAllPanels = () => {
    setShowRecommendations(false);
    setShowDataFetching(false);
    setShowDataManagement(false);
  };

  // Modal management
  const openModal = (modalId: string) => {
    setActiveModal(modalId);
  };

  const closeModal = () => {
    setActiveModal(null);
  };

  // Toast management
  const addToast = (message: string, type: 'success' | 'error' | 'info' | 'warning' = 'info', duration = 5000) => {
    const id = Date.now().toString();
    const toast = { id, message, type, duration };
    
    setToasts(prev => [...prev, toast]);
    
    // Auto-remove toast after duration
    if (duration > 0) {
      setTimeout(() => {
        removeToast(id);
      }, duration);
    }
    
    return id;
  };

  const removeToast = (id: string) => {
    setToasts(prev => prev.filter(toast => toast.id !== id));
  };

  // Clear all toasts
  const clearToasts = () => {
    setToasts([]);
  };

  // Global error handling
  const showError = (message: string) => {
    setGlobalError(message);
    addToast(message, 'error');
  };

  const clearError = () => {
    setGlobalError(null);
  };

  return {
    // Navigation state
    activeTab,

    // Panel state
    showRecommendations,
    showDataFetching,
    showDataManagement,

    // Modal state
    activeModal,

    // Global state
    globalLoading,
    globalError,
    toasts,

    // Navigation actions
    openScreening,

    // Panel actions
    openRecommendations,
    openDataFetching,
    closeAllPanels,

    // Modal actions
    openModal,
    closeModal,

    // Toast actions
    addToast,
    removeToast,
    clearToasts,

    // Global actions
    showError,
    clearError,

    // Setters
    setActiveTab,
    setGlobalLoading,
    setGlobalError
  };
}

// Create global store instance
export const uiStore = createUiStore();