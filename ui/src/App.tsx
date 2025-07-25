import React, { Component } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";

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
      <BrowserRouter>
        <Layout>
          <TopMenu />

          <Routes>
            <Route path="/projects/:id" element={<Project />}/>
            <Route path="/projects" element={<Projects/>} />
            <Route path="/jobs/:id/tokens/:trigger_datetime" element={<Tokens/>} />
            <Route path="/jobs/:job_id/triggers/:trigger_id" element={<Triggers/>} />
            <Route path="/jobs/:id/:tab" element={<Job/>} />
            <Route path="/jobs/:id" element={<Job/>} />
            <Route path="/logs/:task_run_id" element={<TaskLogs/>} />
            <Route path="/schedulers" element={<Schedulers/>} />
            <Route path="/workers/:id" element={<Worker/>} />
            <Route path="/workers" element={<Workers/>} />
            <Route path="/login" element={<Login/>} />
            <Route path="/" element={<Home/>} />
          </Routes>

          <Footer style={{ textAlign: 'center' }}>
            Waterwheel - Version TODO
          </Footer>
        </Layout>
      </BrowserRouter>
    );
  }
}

export default App;

