#!groovy

node ('debian8') { // a docker tag would be better
    stage('checkout') {
        //checkout scm
        //git 'git@github.com:CanalTP/mimirsbrunn.git'
        //git url: 'https://github.com/CanalTP/mimirsbrunn.git', branch: 'master' // use checkout scm when using jenkinsfile 
        git url: 'https://github.com/antoine-de/mimirsbrunn.git', branch: 'jenkinsfile' // tmp
        version = sh script: 'git describe --tags', returnStdout: true
        version = version.replace('\n', '')
        echo "version is ${version}"
        echo "branch name is ${env.BRANCH_NAME}"
    }
    stage('Build & tests') {
        def rust = docker.build('mimir_rust_build', "-f build_image/Dockerfile_build .") // should we add --pull ?
        docker.withServer('par-vm241.srv.canaltp.fr:2375') { //TODO get it dynamicly
            rust.inside() {
                sh "CARGO_HOME=`pwd`/.cargo cargo build --release"
                sh "CARGO_HOME=`pwd`/.cargo cargo test --release"
            }
        }
    }
    stage('docker & packages') {
        docker.withServer('par-vm241.srv.canaltp.fr:2375') {
            docker.image('mimir_rust_build').inside() {
                sh "rm -f *deb" // cleanup old debian packages
                sh "CARGO_HOME=`pwd`/.cargo sh -x ./build_packages.sh"

                // we add the debian packages in the stash
                stash includes: '*.deb', name: 'debian_packages'

                echo "building the docker image"
                echo "version ${version} bob"

                sh "docker build -t bragi:${version} --build-arg BRAGI_BIN=build_packages/bin/bragi ."
                sh "docker tag bragi:$version bragi:master"
                sh "docker tag bragi:master par-vm221.srv.canaltp.fr:5000/bragi:master"
                sh "docker push par-vm221.srv.canaltp.fr:5000/bragi:master"
            }
        }
    }
}

node ('debian8') {
    //docker.image('python:2.7').inside { // TODO I didn't make this work, there is ssh key problems
        stage('setup deploy dependencies') {
            git url: 'git@github.com:CanalTP/fab_mimir.git', branch: 'req' //TODO use master

            unstash 'debian_packages'

            sh """
rm -rf venv/
virtualenv --system-site-packages venv -p python2
. venv/bin/activate
pip install -r requirements.txt
"""
        }
        parallel dev: {
            stage('deploy on dev') {
                sh ". venv/bin/activate && fab dev let:data_refresh=False deploy" //TODO remove data_refresh
            }
        }, bench: {
            stage('deploy on bench') {
                sh ". venv/bin/activate && fab benchmark let:data_refresh=False deploy" //TODO remove data_refresh
                // TODO trigger the geocoding tester job
            }
        }
    //}
}
