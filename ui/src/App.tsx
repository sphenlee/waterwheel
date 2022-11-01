import React, { Component } from "react";
import { BrowserRouter as Router, Switch, Route } from "react-router-dom";

import { Layout, Breadcrumb } from 'antd';

const { Content, Footer } = Layout;

import Home from './pages/Home';
import Job from './pages/Job';
import Login from './pages/Login';
import Project from './pages/Project';
import Projects from './pages/Projects';
import Schedulers from './pages/Schedulers';
import TaskLogs from './pages/TaskLogs';
import Tokens from './pages/Tokens';
import TopMenu from './components/TopMenu'
import Triggers from './pages/Triggers';
import Worker from './pages/Worker';
import Workers from './pages/Workers';

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
            <Route path="/jobs/:id/:tab" component={Job} />
            <Route path="/jobs/:id" component={Job} />
            <Route path="/logs/:task_run_id" component={TaskLogs} />
            <Route path="/schedulers" component={Schedulers} />
            <Route path="/workers/:id" component={Worker} />
            <Route path="/workers" component={Workers} />
            <Route path="/login" component={Login} />
            <Route path="/" component={Home} />
          </Switch>

          <Footer style={{ textAlign: 'center' }}>
            Waterwheel - {VERSION}
          </Footer>
        </Layout>
      </Router>
    );
  }
}

export default App;

