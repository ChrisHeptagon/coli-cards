/** @type {import('next').NextConfig} */
const nextConfig = {
    webpack: (
        config,
        { buildId, dev, isServer, defaultLoaders, nextRuntime, webpack }
      ) => {
        config.devServer = {
            client: {
                hostname: 'localhost',
                pathname: '/ws',
                port: 8500,
                protocol: 'ws',
            }
        }
        return config
      },
    }

module.exports = nextConfig
