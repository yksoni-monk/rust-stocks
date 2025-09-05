import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [stocks, setStocks] = useState([]);
  const [dbStats, setDbStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentView, setCurrentView] = useState('dashboard');

  useEffect(() => {
    fetchInitialData();
  }, []);

  async function fetchInitialData() {
    try {
      setLoading(true);
      const [stocksData, statsData] = await Promise.all([
        invoke('get_all_stocks'),
        invoke('get_database_stats')
      ]);
      setStocks(stocksData);
      setDbStats(statsData);
    } catch (err) {
      setError(`Failed to fetch data: ${err}`);
      console.error('Error fetching data:', err);
    } finally {
      setLoading(false);
    }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      fetchInitialData();
      return;
    }
    
    try {
      const results = await invoke('search_stocks', { query: searchQuery });
      setStocks(results);
    } catch (err) {
      setError(`Search failed: ${err}`);
    }
  }

  async function handleFetchData() {
    try {
      const result = await invoke('fetch_stock_data', {
        stockSymbols: stocks.slice(0, 5).map(s => s.symbol), // First 5 stocks as example
        startDate: '2024-01-01',
        endDate: '2024-12-31'
      });
      alert(result);
    } catch (err) {
      setError(`Data fetch failed: ${err}`);
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600 mx-auto"></div>
          <p className="mt-4 text-gray-600">Loading stock data...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-100 flex items-center justify-center">
        <div className="bg-red-50 border border-red-200 rounded-lg p-6 max-w-md">
          <h2 className="text-red-800 font-semibold mb-2">Error</h2>
          <p className="text-red-600">{error}</p>
          <button 
            onClick={() => { setError(null); fetchInitialData(); }}
            className="mt-4 bg-red-600 text-white px-4 py-2 rounded hover:bg-red-700"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-100">
      {/* Header */}
      <header className="bg-blue-600 text-white shadow-lg">
        <div className="container mx-auto px-4 py-4">
          <div className="flex justify-between items-center">
            <h1 className="text-2xl font-bold">Stock Analysis System</h1>
            <nav className="flex space-x-4">
              <button 
                onClick={() => setCurrentView('dashboard')}
                className={`px-4 py-2 rounded ${currentView === 'dashboard' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Dashboard
              </button>
              <button 
                onClick={() => setCurrentView('stocks')}
                className={`px-4 py-2 rounded ${currentView === 'stocks' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Stocks
              </button>
              <button 
                onClick={() => setCurrentView('analysis')}
                className={`px-4 py-2 rounded ${currentView === 'analysis' ? 'bg-blue-800' : 'hover:bg-blue-700'}`}
              >
                Analysis
              </button>
            </nav>
          </div>
        </div>
      </header>

      <div className="container mx-auto px-4 py-8">
        {currentView === 'dashboard' && (
          <div>
            <h2 className="text-3xl font-bold mb-6">Dashboard</h2>
            
            {/* Database Stats */}
            {dbStats && (
              <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Total Stocks</h3>
                  <p className="text-2xl font-bold text-blue-600">{dbStats.total_stocks.toLocaleString()}</p>
                </div>
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Price Records</h3>
                  <p className="text-2xl font-bold text-green-600">{dbStats.total_price_records.toLocaleString()}</p>
                </div>
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Data Coverage</h3>
                  <p className="text-2xl font-bold text-purple-600">{dbStats.data_coverage_percentage}%</p>
                </div>
                <div className="bg-white p-6 rounded-lg shadow">
                  <h3 className="text-gray-500 text-sm font-medium">Last Update</h3>
                  <p className="text-lg font-semibold text-gray-700">{dbStats.last_update}</p>
                </div>
              </div>
            )}

            {/* Quick Actions */}
            <div className="bg-white rounded-lg shadow p-6">
              <h3 className="text-xl font-semibold mb-4">Quick Actions</h3>
              <div className="flex space-x-4">
                <button 
                  onClick={handleFetchData}
                  className="bg-blue-600 text-white px-6 py-2 rounded hover:bg-blue-700 transition-colors"
                >
                  Fetch Recent Data
                </button>
                <button 
                  onClick={() => setCurrentView('stocks')}
                  className="bg-green-600 text-white px-6 py-2 rounded hover:bg-green-700 transition-colors"
                >
                  View Stocks
                </button>
                <button 
                  onClick={() => setCurrentView('analysis')}
                  className="bg-purple-600 text-white px-6 py-2 rounded hover:bg-purple-700 transition-colors"
                >
                  Start Analysis
                </button>
              </div>
            </div>
          </div>
        )}

        {currentView === 'stocks' && (
          <div>
            <div className="flex justify-between items-center mb-6">
              <h2 className="text-3xl font-bold">Stocks</h2>
              <div className="flex space-x-4">
                <input
                  type="text"
                  placeholder="Search stocks..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
                  className="px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <button 
                  onClick={handleSearch}
                  className="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700"
                >
                  Search
                </button>
              </div>
            </div>

            <div className="bg-white rounded-lg shadow overflow-hidden">
              <div className="overflow-x-auto">
                <table className="min-w-full divide-y divide-gray-200">
                  <thead className="bg-gray-50">
                    <tr>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Symbol</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Company</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Sector</th>
                      <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Actions</th>
                    </tr>
                  </thead>
                  <tbody className="bg-white divide-y divide-gray-200">
                    {stocks.map((stock) => (
                      <tr key={stock.id} className="hover:bg-gray-50">
                        <td className="px-6 py-4 whitespace-nowrap">
                          <div className="text-sm font-medium text-gray-900">{stock.symbol}</div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <div className="text-sm text-gray-900">{stock.company_name}</div>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap">
                          <span className="inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800">
                            {stock.sector || 'N/A'}
                          </span>
                        </td>
                        <td className="px-6 py-4 whitespace-nowrap text-sm font-medium">
                          <button 
                            onClick={() => setCurrentView('analysis')}
                            className="text-blue-600 hover:text-blue-900"
                          >
                            Analyze
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        )}

        {currentView === 'analysis' && (
          <div>
            <h2 className="text-3xl font-bold mb-6">Stock Analysis</h2>
            <div className="bg-white rounded-lg shadow p-6">
              <p className="text-gray-600 mb-4">
                Select a stock from the Stocks page to begin analysis, or this will be enhanced with charts and analysis tools.
              </p>
              <div className="space-y-4">
                <button 
                  onClick={() => setCurrentView('stocks')}
                  className="bg-blue-600 text-white px-6 py-2 rounded hover:bg-blue-700"
                >
                  Go to Stocks
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;