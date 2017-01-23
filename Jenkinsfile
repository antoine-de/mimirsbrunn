#!groovy

node {
    stage 'checkout' {
        checkout scm
        //git 'git@github.com:CanalTP/mimirsbrunn.git'
        //git 'https://github.com/CanalTP/mimirsbrunn.git' // use checkout scm when using jenkinsfile 
        version = sh ('git describe --tags', returnStdout: true)
        echo "version is ${version}"
        echo "branch name is ${env.BRANCH_NAME}"
    }
    stage('Build & tests')  {
        sh 'cargo test --release'
    }
    stage('docker & packages') {
        sh './build_packages.sh'
        //sh 'docker build --build-arg BRAGI_BIN=build_packages/bin/bragi -t bragi:$version .'
    }
}


