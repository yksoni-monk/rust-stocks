import { defineConfig, loadEnv } from 'vite'
import solid from 'vite-plugin-solid'

// https://vite.dev/config/
export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')
  
  return {
    plugins: [solid()],
    server: {
      port: parseInt(env.VITE_PORT) || 5174
    }
  }
})
