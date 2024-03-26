inspect_asm::alloc_zst::try_bumpalo:
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, qword ptr [rcx]
	jb .LBB_2
	ret
.LBB_2:
	mov esi, 1
	xor edx, edx
	jmp qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]