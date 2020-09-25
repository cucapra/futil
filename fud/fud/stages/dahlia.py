from fud.stages import Stage, Step, SourceType, Source

class DahliaStage(Stage):
    def __init__(self, config):
        super().__init__('dahlia', 'futil', config)

    def define(self):
        main = Step(SourceType.Path, SourceType.File)
        main.set_cmd(f'{self.cmd} {{ctx[input_path]}} -b futil --lower')
        return [main]
