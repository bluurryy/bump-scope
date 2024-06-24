inspect_asm::alloc_zst::bumpalo:
	push rax
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, qword ptr [rcx]
	jb .LBB0_1
.LBB0_0:
	pop rcx
	ret
.LBB0_1:
	mov esi, 1
	xor edx, edx
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	test rax, rax
	jne .LBB0_0
	call qword ptr [rip + bumpalo::oom@GOTPCREL]
