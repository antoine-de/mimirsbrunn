#!groovy

node ('docker') {

    def rust = docker.image('jimmycuadra/rust:latest')
    rust.pull()
    rust.inside {
            
        stage('checkout') {   
            //checkout scm
            //git 'git@github.com:CanalTP/mimirsbrunn.git'
            git url: 'https://github.com/CanalTP/mimirsbrunn.git', branch: 'master' // use checkout scm when using jenkinsfile 
            version = sh script: 'git describe --tags', returnStdout: true
            echo "version is ${version}"
            echo "branch name is ${env.BRANCH_NAME}"
        }
        
        stage('Build & tests') {
            sh "CARGO_HOME=`pwd`/.cargo cargo build --release --verbose"
        }

        stage('packages & docker') {
            sh "CARGO_HOME=`pwd`/.cargo ./build_packages.sh"

            // we add the debian packages in the stash
            stash includes: '*.deb', name: 'debian_packages'

            echo "building the docker image"
            sh "docker build --build-arg BRAGI_BIN=build_packages/bin/bragi -t bragi:${version} ."
            sh "docker tag bragi:$version bragi:master"
            sh "docker tag bragi:master par-vm221.srv.canaltp.fr:5000/bragi:master"
            sh "docker push par-vm221.srv.canaltp.fr:5000/bragi:master"
        }
    }
}

def deploy_with_fabric { env ->
    git url: 'https://github.com/CanalTP/fab_mimir.git', branch: 'master'

    unstash 'debian_packages'
    sh 'ls -lha'

    docker.image('python:2.7').inside {
        sh 'pip install -r requirements.txt'
        sh "fab ${env} deploy"
    }
}

node ('docker') {
    stage('deploy on dev') {
        deploy_with_fabric 'dev'
    }
}
