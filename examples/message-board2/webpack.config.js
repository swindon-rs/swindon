var path = require('path');

module.exports = {
    entry: './messageboard.js',
    output: {
        filename: 'bundle.js',
        path: path.resolve(__dirname, 'public/js')
    },
    module: {
        rules: [{
            loader: 'babel-loader',
            test: /\.js$/,
            exclude: /node_modules/,
        }],
    },
    resolve: {
        modules: process.env.NODE_PATH.split(':'),
    },
    resolveLoader: {
        modules: process.env.NODE_PATH.split(':'),
    },
};
