import React, { Component } from "react";
import { HashRouter as Router, Switch, Route } from "react-router-dom";

import { Layout, Breadcrumb } from 'antd';

const { Content } = Layout;

import TopMenu from './components/TopMenu.jsx'
import Home from './pages/Home.jsx';
import Projects from './pages/Projects.jsx';
import Project from './pages/Project.jsx';
import Job from './pages/Job.jsx';
import Tokens from './pages/Tokens.jsx';
import Workers from './pages/Workers.jsx';
import Triggers from './pages/Triggers.jsx';

class App extends Component {
  render() {
    return (
      <Router>
        <Layout>
          <TopMenu />

          <Switch>
            <Route path="/projects/:id" component={Project} />
            <Route path="/projects" component={Projects} />
            <Route path="/jobs/:id/tokens/:trigger_datetime" component={Tokens} />
            <Route path="/jobs/:job_id/triggers/:trigger_id" component={Triggers} />
            <Route path="/jobs/:id" component={Job} />
            <Route path="/workers" component={Workers} />
            <Route path="/" component={Home} />
          </Switch>

        </Layout>
      </Router>
    );
  }
}

export default App;

