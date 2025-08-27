const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');

const isProd = process.env.NODE_ENV === 'production';

const renderer = {
  mode: isProd ? 'production' : 'development',
  target: 'electron-renderer',
  entry: path.resolve(__dirname, 'src/renderer/index.tsx'),
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'renderer.js'
  },
  resolve: {
    extensions: ['.ts', '.tsx', '.js', '.jsx'],
    alias: {
      '@renderer': path.resolve(__dirname, 'src/renderer'),
      '@main': path.resolve(__dirname, 'src/main')
    }
  },
  module: {
    rules: [
      {
        test: /\.(ts|tsx)$/,
        use: 'ts-loader',
        exclude: /node_modules/
      },
      {
        test: /\.css$/,
        use: ['style-loader', 'css-loader', 'postcss-loader']
      }
    ]
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: path.resolve(__dirname, 'src/renderer/index.html'),
      minify: isProd
    })
  ],
  devtool: isProd ? false : 'source-map'
};

const main = {
  mode: isProd ? 'production' : 'development',
  target: 'electron-main',
  entry: path.resolve(__dirname, 'src/main/main.ts'),
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'main.js'
  },
  resolve: {
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/
      }
    ]
  },
  node: {
    __dirname: false,
    __filename: false
  },
  devtool: isProd ? false : 'source-map'
};

const preload = {
  mode: isProd ? 'production' : 'development',
  target: 'electron-preload',
  entry: path.resolve(__dirname, 'src/main/preload.ts'),
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'preload.js'
  },
  resolve: {
    extensions: ['.ts', '.js']
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: 'ts-loader',
        exclude: /node_modules/
      }
    ]
  },
  devtool: isProd ? false : 'source-map'
};

module.exports = [renderer, main, preload];
