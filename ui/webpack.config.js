const webpack = require('webpack');
const HtmlWebPackPlugin = require("html-webpack-plugin");
//const { GitRevisionPlugin } = require('git-revision-webpack-plugin');
//const gitRevisionPlugin = new GitRevisionPlugin({
//      versionCommand: 'describe --always --tags --dirty=-modified'
//  });

module.exports = {
  entry: './src/index.tsx',
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: 'ts-loader',
        exclude: /node_modules/,
      },
      {
        test: /\.html$/,
        use: [
          {
            loader: "html-loader"
          }
        ]
      },
      {
        test: /\.css$/i,
        use: ['style-loader', 'css-loader'],
      },
    ]
  },
  plugins: [
    new HtmlWebPackPlugin({
      template: "./src/index.html",
      filename: "./index.html"
    }),
    //new webpack.DefinePlugin({
    //    'VERSION': JSON.stringify(gitRevisionPlugin.version()),
    //    'COMMITHASH': JSON.stringify(gitRevisionPlugin.commithash()),
    //})
  ],
  resolve: {
    extensions: ['.tsx', '.ts', '.js']
  },
  output: {
    publicPath: '/static/',
  },
};