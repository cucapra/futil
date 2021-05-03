import logging as log

from paramiko import SSHClient
from scp import SCPClient
from pathlib import Path

from fud.stages import Stage, SourceType
from fud.utils import TmpDir
from fud import errors


class HwEmulationStage(Stage):
    def __init__(self, config):
        super().__init__(
            "xclbin",
            "wdb",
            SourceType.Path,
            SourceType.Path,
            config,
            "Runs Vivado hw emulation",
        )

        self.setup_commands = "source /scratch/opt/Xilinx/Vitis/2020.2/settings64.sh && source /opt/xilinx/xrt/setup.sh"

        self.host_cpp = self.config["stages", self.target_stage, "host"]

        self.xrt = (
            Path(self.config["global", "futil_directory"])
            / "fud"
            / "bitstream"
            / "xrt.ini"
        )
        self.device = "xilinx_u50_gen3x16_xdma_201920_3"

        # remote execution
        self.ssh_host = self.config["stages", self.target_stage, "ssh_host"]
        self.ssh_user = self.config["stages", self.target_stage, "ssh_username"]

        self.setup()

    def _define_steps(self, input_data):
        @self.step()
        def check_host_cpp():
            """
            Make sure that `-s wdb.host` is provided
            """
            if self.host_cpp is None:
                raise errors.MissingDynamicConfiguration("wdb.host")

        @self.step()
        def establish_connection() -> SourceType.UnTyped:
            """
            Establish SSH connection
            """
            client = SSHClient()
            client.load_system_host_keys()
            client.connect(self.ssh_host, username=self.ssh_user)
            return client

        @self.step()
        def make_remote_tmpdir(client: SourceType.UnTyped) -> SourceType.String:
            """
            Execution `mktemp -d` on server.
            """
            _, stdout, _ = client.exec_command("mktemp -d")
            return stdout.read().decode("ascii").strip()

        @self.step()
        def send_files(
            client: SourceType.UnTyped,
            tmpdir: SourceType.String,
            xclbin: SourceType.Path,
        ):
            """
            Copy files over ssh channel
            """
            with SCPClient(client.get_transport()) as scp:
                scp.put(xclbin, remote_path=f"{tmpdir}/kernel.xclbin")
                scp.put(self.host_cpp, remote_path=f"{tmpdir}/host.cpp")
                scp.put(self.xrt, remote_path=f"{tmpdir}/xrt.ini")

        @self.step()
        def setup_environment(client: SourceType.UnTyped):
            """
            Source Xilinx scripts
            """

        @self.step()
        def compile_host(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Compile the host code
            """
            _, stdout, stderr = client.exec_command(
                " ".join(
                    [
                        f"cd {tmpdir}",
                        "&&",
                        "g++",
                        "-I/opt/xilinx/xrt/include",
                        "-I/scratch/opt/Xilinx/Vivado/2020.2/include",
                        "-Wall -O0 -g -std=c++14 -fmessage-length=0",
                        "host.cpp",
                        "-o 'host'",
                        "-L/opt/xilinx/xrt/lib -lOpenCL -lpthread -lrt -lstdc++",
                    ]
                )
            )

            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())
            log.debug(stderr.read())

        @self.step()
        def generate_emconfig(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Generate emconfig.json
            """
            _, stdout, stderr = client.exec_command(
                " ".join(
                    [
                        f"cd {tmpdir}",
                        "&&",
                        "/scratch/opt/Xilinx/Vitis/2020.2/bin/emconfigutil",
                        f"--platform {self.device}",
                        "--od .",
                    ]
                )
            )

            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())
            log.debug(stderr.read())

        @self.step()
        def emulate(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Emulation the xclbin
            """
            _, stdout, stderr = client.exec_command(
                " ".join(
                    [
                        f"cd {tmpdir}",
                        "&&",
                        self.setup_commands,
                        "&&",
                        "XCL_EMULATION_MODE=hw_emu",
                        "./host",
                        "kernel.xclbin",
                        self.device,
                    ]
                )
            )

            for chunk in iter(lambda: stdout.readline(2048), ""):
                log.debug(chunk.strip())
            log.debug(stderr)

        @self.step()
        def download_wdb(
            client: SourceType.UnTyped,
            tmpdir: SourceType.String,
        ) -> SourceType.Stream:
            """
            Download xclbin file
            """
            local_tmpdir = TmpDir()
            wdb_path = Path(local_tmpdir.name) / "kernel.wdb"
            with SCPClient(client.get_transport()) as scp:
                scp.get(
                    f"{tmpdir}/xilinx_u50_gen3x16_xdma_201920_3-0-kernel.wdb",
                    local_path=str(wdb_path),
                )
            return wdb_path.open("rb")

        @self.step()
        def cleanup(client: SourceType.UnTyped, tmpdir: SourceType.String):
            """
            Close SSH Connection and cleanup temporaries.
            """
            client.exec_command("rm -r {tmpdir}")
            client.close()

        check_host_cpp()
        client = establish_connection()
        tmpdir = make_remote_tmpdir(client)
        send_files(client, tmpdir, input_data)
        compile_host(client, tmpdir)
        generate_emconfig(client, tmpdir)
        emulate(client, tmpdir)
        wdb = download_wdb(client, tmpdir)
        cleanup(client, tmpdir)

        return wdb
