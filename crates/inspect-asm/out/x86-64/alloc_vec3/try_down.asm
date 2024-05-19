inspect_asm::alloc_vec3::try_down:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	and rax, -4
	add rax, -12
	cmp rax, qword ptr [rcx + 8]
	jb .LBB_2
	mov qword ptr [rcx], rax
	test rax, rax
	je .LBB_2
.LBB_4:
	mov ecx, dword ptr [rsi + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rsi]
	mov qword ptr [rax], rcx
	pop rbx
	ret
.LBB_2:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<A,_,_,_>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	test rax, rax
	jne .LBB_4
	xor eax, eax
	pop rbx
	ret