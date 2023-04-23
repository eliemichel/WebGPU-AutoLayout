const path = require('path');
const webpack = require('webpack');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

module.exports = {
    entry: {
        index: './src/index.js',
    },
    output: {
        filename: '[name].bundle.js',
        path: path.resolve(__dirname, 'dist'),
        clean: true,
    },
    plugins: [
        new HtmlWebpackPlugin({
            title: 'WebGPU Autolayout',
        }),
        new WasmPackPlugin({
            crateDirectory: path.resolve(__dirname, "../wgsl-parser-wasm"),
            outDir: path.resolve(__dirname, "../wgsl-parser-wasm/pkg"),
            watchDirectories: [
                path.resolve(__dirname, "../wgsl-parser"),
            ],
        }),
    ],
    module: {
        rules: [
            {
                test: /\.css$/i,
                use: ['style-loader', 'css-loader'],
            },
            {
                test: /\.wasm/i,
                type: "webassembly/async",
            },
        ],
    },
    experiments: {
        asyncWebAssembly: true,
    },
};
